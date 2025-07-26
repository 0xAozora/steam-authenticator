// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod util;

use reqwest;
use serde::{Deserialize, Serialize};
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::sync::Mutex;
use std::time::SystemTime;
use std::{collections::HashSet, fs};
use steamguard::token::Tokens;
use tauri::{command, State};
use util::conf_to_u32;

// Define a structure to match the JSON structure.
#[derive(Serialize, Deserialize, Debug)]
struct MaFile {
    shared_secret: String,
    serial_number: String,
    revocation_code: String,
    uri: String,
    server_time: u64,
    account_name: String,
    token_gid: String,
    identity_secret: String,
    secret_1: String,
    status: u8,
    device_id: String,
    fully_enrolled: bool,
    Session: Session,
}

#[derive(Serialize, Deserialize, Debug)]
struct Session {
    SteamID: u64,
    AccessToken: Option<String>,
    RefreshToken: Option<String>,

    SessionID: Option<String>,
    SteamLogin: Option<String>,
    SteamLoginSecure: Option<String>,
    OAuthToken: Option<String>,
}

// Store the list of names in Tauri's application state.
struct AppState {
    accounts: Mutex<HashMap<String, steamguard::SteamGuardAccount>>,
    confirmationCache: Mutex<HashMap<String, Vec<steamguard::Confirmation>>>,
}

static DIRECTORY_PATH: &'static str = "./accounts";

// Function to read and parse JSON files.
fn load_accounts() -> Result<HashMap<String, steamguard::SteamGuardAccount>, String> {
    let path = Path::new(DIRECTORY_PATH);

    if !path.exists() || !path.is_dir() {
        return Err("Provided path is not a valid directory.".into());
    }

    let mut accounts = HashMap::new();

    // Read the directory and process `.json` files.
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.extension().map_or(false, |ext| ext == "maFile") {
                // Read the content of the file
                if let Ok(file) = fs::File::open(file_path.clone()) {
                    // Parse the JSON

                    match serde_json::from_reader::<_, MaFile>(&file) {
                        Ok(ma_file) => {
                            let tokens: Option<steamguard::token::Tokens>;
                            if ma_file.Session.AccessToken.is_some()
                                && ma_file.Session.RefreshToken.is_some()
                            {
                                tokens = Some(steamguard::token::Tokens::new(
                                    ma_file.Session.AccessToken.unwrap(),
                                    ma_file.Session.RefreshToken.unwrap(),
                                ))
                            } else {
                                tokens = None
                            }

                            let account = steamguard::SteamGuardAccount {
                                account_name: ma_file.account_name,
                                steam_id: ma_file.Session.SteamID,
                                serial_number: ma_file.serial_number,
                                revocation_code: steamguard::SecretString::new(
                                    ma_file.revocation_code,
                                ),
                                shared_secret:
                                    steamguard::token::TwoFactorSecret::parse_shared_secret(
                                        ma_file.shared_secret,
                                    )
                                    .unwrap(),
                                token_gid: ma_file.token_gid,
                                identity_secret: steamguard::SecretString::new(
                                    ma_file.identity_secret,
                                ),
                                uri: steamguard::SecretString::new(ma_file.uri),
                                device_id: ma_file.device_id,
                                secret_1: steamguard::SecretString::new(ma_file.secret_1),
                                tokens: tokens,
                            };

                            match save_account(&account) {
                                Ok(()) => {
                                    let _ = fs::remove_file(file_path);
                                }
                                Err(_) => {}
                            }

                            accounts.insert(account.account_name.clone(), account);
                        }
                        Err(e) => eprintln!("Failed to parse JSON: {}", e),
                    };
                }
            } else if file_path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(file) = fs::File::open(file_path) {
                    // Parse the JSON
                    match serde_json::from_reader::<_, steamguard::SteamGuardAccount>(&file) {
                        Ok(account) => {
                            accounts.insert(account.account_name.clone(), account);
                        }
                        Err(e) => eprintln!("Failed to parse JSON: {}", e),
                    };
                }
            }
        }
    } else {
        return Err("Failed to read the directory.".into());
    }

    Ok(accounts)
}

fn save_account(account: &steamguard::SteamGuardAccount) -> Result<(), String> {
    let path = Path::new(DIRECTORY_PATH);

    if !path.exists() || !path.is_dir() {
        return Err("Provided path is not a valid directory.".into());
    }

    // Open the file in write-only mode, creating it if it doesn’t exist,
    // and truncating it (clearing contents) if it does.
    match fs::File::create(path.join(format!("{}.json", account.steam_id))) {
        Ok(file) => {
            if let Err(e) = serde_json::to_writer(file, account) {
                return Err(e.to_string());
            }
        }
        Err(e) => return Err(e.to_string()),
    }

    Ok(())
}

#[tauri::command]
fn get_stored_names(state: State<AppState>) -> Vec<String> {
    let mut names: Vec<String> = state.accounts.lock().unwrap().keys().cloned().collect();
    names.sort();
    return names;
}

#[tauri::command]
fn get_code(state: State<AppState>, name: &str) -> String {
    return state
        .accounts
        .lock()
        .unwrap()
        .get(name)
        .unwrap()
        .generate_code(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
}

#[tauri::command]
fn login(state: State<AppState>, name: &str, password: &str) -> Result<(), String> {
    let mut map = state.accounts.lock().unwrap();
    let account = map.get_mut(name).unwrap();

    let http_client = reqwest::blocking::Client::builder();
    let http_client = http_client.build().unwrap();
    let transport = steamguard::transport::WebApiTransport::new(http_client);

    let mut login = steamguard::UserLogin::new(transport, build_device_details());

    let confirmation_methods;
    loop {
        match login.begin_auth_via_credentials(&account.account_name, password) {
            Ok(methods) => {
                confirmation_methods = methods;
                break;
            }
            Err(steamguard::LoginError::TooManyAttempts) => {
                return Err(steamguard::LoginError::TooManyAttempts.to_string());
            }
            Err(steamguard::LoginError::BadCredentials) => {
                return Err(steamguard::LoginError::BadCredentials.to_string());
            }
            Err(steamguard::LoginError::SessionExpired) => {
                return Err(steamguard::LoginError::SessionExpired.to_string());
            }
            Err(err) => {
                return Err(err.to_string());
            }
        }
    }

    let mut attempts = 0;

    for method in confirmation_methods {
        match method.confirmation_type {
			steamguard::protobufs::steammessages_auth_steamclient::EAuthSessionGuardType::k_EAuthSessionGuardType_DeviceCode => {
				loop {

                    let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                    let code = account.generate_code(time);
                    
                    attempts += 1;
					match login.submit_steam_guard_code(method.confirmation_type, code) {
						Ok(_) => break,
						Err(err) => {
							//error!("Failed to submit code: {}", err);

							match err {
								steamguard::userlogin::UpdateAuthSessionError::TooManyAttempts
								| steamguard::userlogin::UpdateAuthSessionError::SessionExpired
								| steamguard::userlogin::UpdateAuthSessionError::InvalidGuardType => {
									return Err(err.to_string());
								}
								_ => {}
							}
							
							//debug!("Attempts: {}/3", attempts);
							if attempts >= 3 {
								//error!("Too many failed attempts. Aborting.");
								return Err(err.to_string());
							}
						}
					}
				}
			}
			_ => {
				//warn!("Unknown confirmation method: {:?}", method);
				continue;
			}
		}
        break;
    }

    if attempts == 0 {
        return Err("Login not supported".to_string());
    }

    match login.poll_until_tokens() {
        Ok(tokens) => {
            account.set_tokens(tokens);
            let _ = save_account(account);
        }
        Err(err) => {
            return Err(err.to_string());
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct ConfirmationInfo {
    pub conf_type: u32,
    pub type_name: String,
    pub id: String,
    /// Trade offer ID or market transaction ID
    pub creator_id: String,
    pub nonce: String,
    pub creation_time: u64,
    pub cancel: String,
    pub accept: String,
    pub icon: Option<String>,
    pub multi: bool,
    pub headline: String,
    pub summary: Vec<String>,
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn get_confirmations(
    state: State<AppState>,
    name: &str,
    refresh: bool,
) -> Result<Vec<ConfirmationInfo>, String> {
    if !refresh {
        let map = state.confirmationCache.lock().unwrap();
        if let Some(confs) = map.get(name) {
            if confs.len() != 0 {
                return Ok(conf_to_info(confs));
            }
        }
    }

    let mut map = state.accounts.lock().unwrap();
    let account = map.get_mut(name).unwrap();
    //let account = binding.as_mut().unwrap();

    if !account.is_logged_in() {
        return Err("Not logged in".to_string());
    }

    let http_client = reqwest::blocking::Client::builder();
    let http_client = http_client.build().unwrap();
    let transport = steamguard::transport::WebApiTransport::new(http_client);

    let mut count: u8 = 0;
    loop {
        let confirmer = steamguard::Confirmer::new(transport.clone(), &account);

        match confirmer.get_confirmations() {
            Ok(confs) => {
                let confirmation_infos = conf_to_info(&confs);

                state
                    .confirmationCache
                    .lock()
                    .unwrap()
                    .insert(account.account_name.clone(), confs);

                return Ok(confirmation_infos);
            }
            Err(steamguard::ConfirmerError::InvalidTokens) => {
                if count == 1 {
                    account.tokens = None;
                    let _ = save_account(account);
                    return Err("Can't refresh tokens".to_string());
                }
                count += 1;

                let client = steamguard::steamapi::AuthenticationClient::new(transport.clone());
                let mut refresher = steamguard::refresher::TokenRefresher::new(client);

                let tokens = account.tokens.as_mut().unwrap();
                match refresher.refresh(account.steam_id, &tokens) {
                    Ok(token) => tokens.set_access_token(token),
                    Err(err) => return Err(err.to_string()),
                }

                //return Err("Invalid Tokens".to_string());
                //crate::do_login(transport.clone(), &mut account, args.password.clone())?;
            }
            Err(err) => {
                //error!("Failed to get confirmations: {}", err);
                return Err(err.to_string());
            }
        }
    }
}

fn conf_to_info(confs: &Vec<steamguard::Confirmation>) -> Vec<ConfirmationInfo> {
    let mut confirmation_infos: Vec<ConfirmationInfo> = Vec::new();

    confs.iter().for_each(|confirmation| {
        confirmation_infos.push(ConfirmationInfo {
            conf_type: conf_to_u32(&confirmation.conf_type),
            type_name: confirmation.type_name.clone(),
            id: confirmation.id.clone(),
            /// Trade offer ID or market transaction ID
            creator_id: confirmation.creator_id.clone(),
            nonce: confirmation.nonce.clone(),
            creation_time: confirmation.creation_time,
            cancel: confirmation.cancel.clone(),
            accept: confirmation.accept.clone(),
            icon: confirmation.icon.clone(),
            multi: confirmation.multi,
            headline: confirmation.headline.clone(),
            summary: confirmation.summary.clone(),
        })
    });

    return confirmation_infos;
}

#[tauri::command]
fn handle_confirmation(
    state: State<AppState>,
    name: &str,
    id: &str,
    accept: bool,
) -> Result<(), String> {
    let mut map = state.accounts.lock().unwrap();
    let account = map.get_mut(name).unwrap();

    let mut map = state.confirmationCache.lock().unwrap();
    let confirmations = map.get_mut(name).unwrap();

    if let Some(index) = confirmations.iter().position(|conf| conf.id == id) {
        let confirmation = unsafe { confirmations.get_unchecked(index) };

        let http_client = reqwest::blocking::Client::builder();
        let http_client = http_client.build().unwrap();
        let transport = steamguard::transport::WebApiTransport::new(http_client);

        let confirmer = steamguard::Confirmer::new(transport, &account);

        let result: Result<(), steamguard::ConfirmerError>;
        if accept {
            result = confirmer.accept_confirmation(confirmation);
        } else {
            result = confirmer.deny_confirmation(confirmation);
        }

        match result {
            Ok(()) => {
                confirmations.remove(index);
            }
            Err(err) => return Err(err.to_string()),
        }
    } else {
        return Err("Confirmation does not exist.".to_string());
    }

    Ok(())
}

fn build_device_details() -> steamguard::DeviceDetails {
    steamguard::DeviceDetails {
		friendly_name: format!(
			"{} (Steam Authenticator)",
			gethostname::gethostname()
				.into_string()
				.expect("failed to get hostname")
		),
		platform_type: steamguard::protobufs::steammessages_auth_steamclient::EAuthTokenPlatformType::k_EAuthTokenPlatformType_MobileApp,
		os_type: -500,
		gaming_device_type: 528,
	}
}

fn main() {
    // Load the JSON names at the start of the program.
    let accounts = load_accounts().unwrap_or(HashMap::new());
    let cache = HashMap::new();

    tauri::Builder::default()
        .manage(AppState {
            accounts: Mutex::new(accounts),
            confirmationCache: Mutex::new(cache),
        })
        .invoke_handler(tauri::generate_handler![
            get_stored_names,
            get_code,
            get_confirmations,
            handle_confirmation,
            login,
        ])
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

import { invoke } from "@tauri-apps/api/core";

import './timer-ring/timer-ring';

let accountElement: Element | null;

window.addEventListener("DOMContentLoaded", () => {
  main()
});

async function main() {

  try {
    const names = (await invoke('get_stored_names')) as string[];

    const list = document.getElementsByTagName('ul')[0];
    list.addEventListener('click', loadAccount);

    // Clear the list
    list.innerHTML = '';

    // Add filenames to the list
    names.forEach((name) => {
      const listItem = document.createElement('li');
      listItem.textContent = name;
      list.appendChild(listItem);
    });

    accountElement = list.firstElementChild;
    accountElement?.dispatchEvent(new Event('click',{ bubbles: true }))

    document.getElementsByTagName('timer-ring')[0].addEventListener('animationiteration', () => loadCode(accountElement?.textContent!));
    document.getElementById('code')?.addEventListener('click', (e) => navigator.clipboard.writeText((e.target as HTMLElement).textContent!))
    document.getElementById('confirmations')?.addEventListener('click', handleEvent);
  } catch (error) {
    console.error("Failed to load JSON files:", error);
    alert("Error: " + error);
  }
}

function handleEvent(e: Event) {

  e.stopPropagation();
  const target = e.composedPath()[0] as HTMLElement;

  let accept: boolean = false;;
  switch(target.id) {
    case 'accept':
      accept = true;
    case 'cancel':
      break;
    default:
      return;
  }

  const span = target.parentElement!;
  span.innerHTML = '';
  span.setAttribute('class', 'loader');

  const id = span.closest('div[id]')!.id
  const div = e.currentTarget! as HTMLElement
  invoke('handle_confirmation', {name: accountElement!.textContent, id: id, accept: accept})
  .then(() => {
    renderConfirmationResult(div, id)
  })
  .catch((error) => {
    renderConfirmationResult(div, id, error)
  });
}

function renderConfirmationResult(div: HTMLElement, id: string, error?: any) {
  const element = div.querySelector('[id="'+id+'"]');
  if(element) {
    const span = element.querySelector('span');
    if(span) {
      let attribute = 'ok';
      if(error) attribute = 'error';
      else error = 'Ok';
      span.setAttribute('class', attribute);
      span.textContent = error;
    }
  }
}

async function loadAccount(e: Event) {

  const originalTarget = e.composedPath()[0] as HTMLElement;
  if(originalTarget.tagName != 'LI') return;

  accountElement?.removeAttribute('checked');
  originalTarget.setAttribute('checked', '');
  accountElement = originalTarget;

  await loadCode(originalTarget.textContent!)

  // Confirmations
  getConfirmations();
}

async function loadCode(account: string) {
  const code = (await invoke('get_code', {name: account})) as string;
  const element = document.getElementById('code') as HTMLElement;
  element.textContent = code;
}

interface Confirmation {
  id: string
  icon: string | undefined
  headline: string
  summary: string[]
  accept: string
  cancel: string
}

function getConfirmations() {
  const div = document.getElementById('confirmations')! as HTMLElement;

  const span = document.createElement('span');
  span.setAttribute('class', 'loader');
  div.innerHTML = '';
  div.appendChild(span);

  invoke('get_confirmations', {name: accountElement!.textContent, refresh: false}).then((confirmations) => renderConfirmations(div, confirmations as Confirmation[]))
  .catch((error) => {
    console.error(error)
    div.innerHTML = '';
    renderLogin(div, error)
  });
}

function renderConfirmations(div: HTMLElement, confirmations: Confirmation[]) {
  div.innerHTML = '';
  if(!confirmations.length) {
    div.innerHTML = 'No confirmations';
    return
  }

  confirmations.forEach((conf) => {
    div.appendChild(buildConfirmation(conf))
  });
}

function buildConfirmation(conf: Confirmation): HTMLElement {
  const div = document.createElement('div');
  div.id = conf.id;
  if(conf.icon) {
    const img = document.createElement('img');
    img.src = conf.icon;
    div.appendChild(img);
  }
  
  const content = document.createElement('div');
  const headline = document.createElement('b');
  headline.textContent = conf.headline;
  content.appendChild(headline);
  
  if(conf.summary.length) {
    const p = document.createElement('p');
    conf.summary.forEach((s) => {
      p.append(s);
      p.appendChild(document.createElement('br'));
    });
    content.appendChild(p);
  }
  

  const span = document.createElement('span');
  span.setAttribute('class', 'buttons')
  const acceptButton = document.createElement('button');
  acceptButton.id = 'accept';
  acceptButton.textContent = conf.accept;
  span.appendChild(acceptButton);

  const cancelButton = document.createElement('button');
  cancelButton.id = 'cancel';
  cancelButton.textContent = conf.cancel;
  span.appendChild(cancelButton);

  content.appendChild(span);
  div.appendChild(content);

  return div;
}

function renderLogin(div: HTMLElement, err: string) {
  const input = document.createElement('input');
  input.setAttribute('placeholder', 'Password');

  const button = document.createElement('button');
  button.textContent = 'Login';

  button.addEventListener('click', login);
  
  div.appendChild(input);
  div.appendChild(button);

  console.log(err);
}

async function login(_e: Event) {

  const input = document.getElementsByTagName('input')[0];

  invoke('login', {name: accountElement!.textContent, password: input.value})
  .then(() => getConfirmations())
  .catch((error) => console.log(error));

}

// Attach the button's click event
// document.getElementById("load-files").addEventListener("click", loadJsonFiles);
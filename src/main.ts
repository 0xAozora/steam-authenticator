import { invoke } from "@tauri-apps/api/core";

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

    const div = document.getElementById('confirmations');
    div?.addEventListener('click', handleEvent, true);
  } catch (error) {
    console.error("Failed to load JSON files:", error);
    alert("Error: " + error);
  }
}

function handleEvent(e: Event) {
  console.log(e);
}

async function loadAccount(e: Event) {

  const originalTarget = e.composedPath()[0] as HTMLElement;
  if(originalTarget.tagName != 'LI') return;

  accountElement?.removeAttribute('checked');
  originalTarget.setAttribute('checked', '');
  accountElement = originalTarget;

  const code = (await invoke('get_code', {name: originalTarget.textContent})) as string;
  const p = document.getElementsByTagName('p')[0];
  p.textContent = code;

  // Confirmations
  getConfirmations();
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
  invoke('get_confirmations', {name: accountElement!.textContent, refresh: false}).then((confirmations) => renderConfirmations(confirmations as Confirmation[]))
  .catch((error) => {
    console.error(error)
    const div = document.getElementById('confirmations');
    div!.innerHTML = '';
    renderLogin(div!, error)
  });
}

function renderConfirmations(confirmations: Confirmation[]) {
  const div = document.getElementById('confirmations')!;
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
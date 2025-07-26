import styles from './timer-ring.css?inline';


class TimerRing extends HTMLElement {

    constructor() {
      super();
      
    }

    connectedCallback() {
      // Shadow DOM to encapsulate styles
      const shadow = this.attachShadow({ mode: 'open' });
      
      // Initial render
      shadow.innerHTML = `
          <svg viewBox="-50 -50 100 100" stroke-width="10">
              <circle r="45" pathLength="1"></circle>
          </svg>

          <style>
          ${styles}
          </style>
      `;

      const now = new Date();
      const ms = now.getSeconds() * 1000 + now.getMilliseconds();

      // Calculate the delay until the next half-minute (either :00 or :30)
      const msRemaining = (ms < 30_000) ? (30_000 - ms) : (60_000 - ms);
      this.setAttribute('style','animation-delay: -' + (30_000 - msRemaining) + 'ms;')
    }
}

customElements.define('timer-ring', TimerRing);

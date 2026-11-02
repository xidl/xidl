// Starlight owns the resolved theme. Scalar publishes its colors through body classes.
const synchronize = () => {
  const body = document.body;
  if (!body.matches('.light-mode, .dark-mode')) return;
  const dark = document.documentElement.dataset.theme === 'dark';
  if (body.classList.contains('dark-mode') !== dark) {
    body.classList.toggle('dark-mode', dark);
  }
  if (body.classList.contains('light-mode') === dark) {
    body.classList.toggle('light-mode', !dark);
  }
};

const observer = new MutationObserver(synchronize);
const connect = () => {
  observer.disconnect();
  observer.observe(document.documentElement, {
    attributeFilter: ['data-theme'],
    attributes: true,
  });
  observer.observe(document.body, {
    attributeFilter: ['class'],
    attributes: true,
  });
  synchronize();
};
connect();
document.addEventListener('astro:after-swap', connect);

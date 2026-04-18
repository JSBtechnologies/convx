import { defineConfig } from '#q-app/wrappers';

export default defineConfig(() => {
  return {
    boot: [],

    css: ['app.scss'],

    extras: ['material-symbols-rounded'],

    build: {
      target: {
        browser: ['es2022', 'chrome110', 'safari16'],
        node: 'node20',
      },
      vueRouterMode: 'history',
      typescript: {
        strict: true,
      },
    },

    devServer: {
      port: 1420,
      open: false,
    },

    framework: {
      config: {
        dark: true,
        notify: {
          position: 'top-right',
          timeout: 3000,
        },
      },
      iconSet: 'material-symbols-rounded',
      plugins: ['Notify', 'Dialog', 'Loading'],
    },

    animations: [],
  };
});

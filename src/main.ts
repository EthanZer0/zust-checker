import { mount } from 'svelte';
import App from './App.svelte';
import { applyTheme } from './lib/theme.svelte';
import './app.css';

// 挂载前应用已保存的主题，避免闪烁
applyTheme();

const app = mount(App, { target: document.getElementById('app')! });

export default app;

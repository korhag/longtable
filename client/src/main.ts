import { mount } from 'svelte';
import '@fontsource/cinzel/500.css';
import '@fontsource/cinzel/700.css';
import '@fontsource/inter/400.css';
import '@fontsource/inter/600.css';
import App from './App.svelte';
import './app.css';

const target = document.getElementById('app');
if (target) mount(App, { target });

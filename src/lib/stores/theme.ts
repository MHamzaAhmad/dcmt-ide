import { writable } from 'svelte/store';
import { browser } from '$app/environment';

type Theme = 'light' | 'dark';

function createThemeStore() {
	const { subscribe, set, update } = writable<Theme>('light');

	return {
		subscribe,
		set,
		toggle: () => update(theme => theme === 'light' ? 'dark' : 'light'),
		init: () => {
			if (!browser) return;

			// Check localStorage first
			const stored = localStorage.getItem('theme') as Theme | null;
			if (stored) {
				set(stored);
				document.documentElement.classList.toggle('dark', stored === 'dark');
				return;
			}

			// Check system preference
			const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
			const theme = prefersDark ? 'dark' : 'light';
			set(theme);
			document.documentElement.classList.toggle('dark', theme === 'dark');
		}
	};
}

export const theme = createThemeStore();

// Subscribe to theme changes and update DOM and localStorage
if (browser) {
	theme.subscribe(value => {
		localStorage.setItem('theme', value);
		document.documentElement.classList.toggle('dark', value === 'dark');
	});
}
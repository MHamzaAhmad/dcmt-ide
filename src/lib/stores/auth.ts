/**
 * AuthStore - Manages authentication state using Clerk
 * Web platform only - Desktop bypasses authentication
 */

import { writable, derived, get } from 'svelte/store';
import { browser } from '$app/environment';
import { isTauri } from '$lib/utils/platform';
import type { Clerk } from '@clerk/clerk-js';

export interface AuthState {
    isReady: boolean;
    isLoading: boolean;
    isAuthenticated: boolean;
    user: any | null;
    session: any | null;
    error: string | null;
}

function createAuthStore() {
    const initialState: AuthState = {
        isReady: false,
        isLoading: false,
        isAuthenticated: false,
        user: null,
        session: null,
        error: null
    };

    const { subscribe, set, update } = writable<AuthState>(initialState);

    let clerkInstance: Clerk | null = null;

    const authStore = {
        subscribe,

        /**
         * Initialize authentication
         * For desktop: immediately mark as authenticated
         * For web: initialize Clerk and check session
         */
        async initialize(): Promise<void> {
            if (!browser) {
                console.log('AuthStore: Not in browser, skipping initialization');
                return;
            }

            // Desktop always authenticated
            if (isTauri()) {
                console.log('AuthStore: Desktop platform detected, bypassing authentication');
                update(state => ({
                    ...state,
                    isReady: true,
                    isAuthenticated: true,
                    user: { id: 'desktop-user', firstName: 'Desktop', lastName: 'User' } as any
                }));
                return;
            }

            // Web platform - Initialize Clerk
            console.log('AuthStore: Web platform detected, initializing Clerk');

            update(state => ({ ...state, isLoading: true }));

            try {
                const publishableKey = import.meta.env.VITE_CLERK_PUBLISHABLE_KEY;

                if (!publishableKey) {
                    throw new Error('Missing VITE_CLERK_PUBLISHABLE_KEY environment variable');
                }

                // Dynamic import to avoid loading Clerk on desktop
                const ClerkJS = await import('@clerk/clerk-js');

                clerkInstance = new ClerkJS.Clerk(publishableKey);

                await clerkInstance.load();

                // Check authentication status
                const isSignedIn = clerkInstance.isSignedIn

                if (isSignedIn) {
                    // User is authenticated
                    console.log('AuthStore: User authenticated', clerkInstance.user);

                    update(state => ({
                        ...state,
                        isReady: true,
                        isLoading: false,
                        isAuthenticated: true,
                        user: clerkInstance!.user,
                        session: clerkInstance!.session,
                        error: null
                    }));

                    // Listen for auth changes
                    clerkInstance.addListener((resources: any) => {
                        update(state => ({
                            ...state,
                            user: resources.user,
                            session: resources.session,
                            isAuthenticated: resources.user !== null
                        }));
                    });
                } else {
                    // User not authenticated - redirect to sign-in
                    console.log('AuthStore: User not authenticated, redirecting to sign-in');

                    const signInUrl = import.meta.env.VITE_CLERK_SIGN_IN_URL;

                    if (!signInUrl) {
                        throw new Error('Missing VITE_CLERK_SIGN_IN_URL environment variable');
                    }

                    // Set state before redirect
                    update(state => ({
                        ...state,
                        isReady: true,
                        isLoading: false,
                        isAuthenticated: false,
                        error: null
                    }));

                    // Redirect to sign-in
                    window.location.href = signInUrl;
                }
            } catch (error) {
                console.error('AuthStore: Initialization failed:', error);

                const errorMessage = error instanceof Error ? error.message : 'Authentication initialization failed';

                update(state => ({
                    ...state,
                    isReady: true,
                    isLoading: false,
                    isAuthenticated: false,
                    error: errorMessage
                }));

                // Emit error event through EventStore if available
                if (browser && typeof window !== 'undefined') {
                    const { eventStore } = await import('./events');
                    eventStore.events.authError(errorMessage);
                }
            }
        },

        /**
         * Check current authentication status
         */
        async checkAuth(): Promise<boolean> {
            // Desktop always authenticated
            if (isTauri()) {
                return true;
            }

            // Web - check Clerk session
            if (!clerkInstance) {
                await authStore.initialize();
            }

            const state = get({ subscribe });
            return state.isAuthenticated;
        },

        /**
         * Sign out the current user (web only)
         */
        async signOut(): Promise<void> {
            if (isTauri()) {
                console.log('AuthStore: Cannot sign out on desktop platform');
                return;
            }

            if (!clerkInstance) {
                console.error('AuthStore: Clerk not initialized');
                return;
            }

            try {
                await clerkInstance.signOut();

                update(state => ({
                    ...state,
                    isAuthenticated: false,
                    user: null,
                    session: null
                }));

                // Redirect to sign-in
                const signInUrl = import.meta.env.VITE_CLERK_SIGN_IN_URL;
                if (signInUrl) {
                    window.location.href = signInUrl;
                }
            } catch (error) {
                console.error('AuthStore: Sign out failed:', error);
            }
        },

        /**
         * Get authentication token for API calls
         * For desktop: returns mock token
         * For web: returns JWT from Clerk session
         */
        async getToken(): Promise<string | null> {
            // Desktop always returns mock token
            if (isTauri()) {
                return 'desktop-mock-token';
            }

            // Web - get token from Clerk session
            if (!clerkInstance) {
                console.warn('AuthStore: Clerk not initialized, cannot get token');
                return null;
            }

            const state = get({ subscribe });
            if (!state.isAuthenticated || !state.session) {
                console.warn('AuthStore: User not authenticated, cannot get token');
                return null;
            }

            try {
                // Get JWT token from Clerk session
                const token = await clerkInstance.session?.getToken();
                return token || null;
            } catch (error) {
                console.error('AuthStore: Failed to get token:', error);
                return null;
            }
        },

        /**
         * Get current state
         */
        getCurrentState(): AuthState {
            return get({ subscribe });
        },

        /**
         * Destroy and cleanup
         */
        destroy(): void {
            if (clerkInstance) {
                clerkInstance = null;
            }

            set(initialState);
        }
    };

    return authStore;
}

export const authStore = createAuthStore();

// Derived stores for convenient access
export const isAuthenticated = derived(authStore, $auth => $auth.isAuthenticated);
export const currentUser = derived(authStore, $auth => $auth.user);
export const isAuthLoading = derived(authStore, $auth => $auth.isLoading);
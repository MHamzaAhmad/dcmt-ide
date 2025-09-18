import { writable, get } from 'svelte/store';
import { billingAPI } from '$lib/api/billing';

export interface BillingState {
    isReady: boolean;
    hasActiveSubscription: boolean;
    benefits: BenefitInfo[];
    lastFetchedAt: number | null;
    error: string | null;
}

export interface BenefitInfo {
    id: string;
    benefit_id: string;
    benefit_type: string;
    description: string;
    metadata: any;
}

function createBillingStore() {
    const { subscribe, set, update } = writable<BillingState>({
        isReady: false,
        hasActiveSubscription: false,
        benefits: [],
        lastFetchedAt: null,
        error: null,
    });

    let initialized = false;

    return {
        subscribe,
        async initializeOnce(): Promise<void> {
            if (initialized) return;
            try {
                const res = await billingAPI.getLimits();
                update((s) => ({
                    ...s,
                    isReady: true,
                    hasActiveSubscription: res.has_active_subscription,
                    benefits: res.benefits,
                    lastFetchedAt: Date.now(),
                    error: null,
                }));
                initialized = true;
            } catch (e: any) {
                // Defaults when API fails or returns not found
                update((s) => ({
                    ...s,
                    isReady: true,
                    hasActiveSubscription: false,
                    benefits: [],
                    lastFetchedAt: Date.now(),
                    error: e?.message || 'Failed to load limits',
                }));
                initialized = true;
            }
        },
        getCurrentState() { return get({ subscribe }); },
        hasBenefit(key: string): boolean {
            const state = get({ subscribe });
            return state.benefits.some((b) => b.benefit_type === key || b.description?.toLowerCase().includes(key));
        },
        async upgrade(): Promise<void> {
            if (typeof window === 'undefined') return;
            // Open a blank tab synchronously to avoid popup blockers
            const popup = window.open('', '_blank', 'noopener,noreferrer');
            try {
                const { url } = await billingAPI.createCheckoutSession();
                if (popup) {
                    try { (popup as any).opener = null; } catch {}
                    popup.location.href = url;
                } else {
                    // Fallback if the popup was blocked
                    window.location.href = url;
                }
            } catch (e) {
                console.error('Failed to create checkout session', e);
                // Close the blank popup on failure
                if (popup && !popup.closed) popup.close();
            }
        },
    };
}

export const billingStore = createBillingStore();

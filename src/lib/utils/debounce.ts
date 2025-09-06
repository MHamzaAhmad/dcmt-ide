/**
 * Creates a debounced version of a function that delays invoking the function until after
 * a specified delay has passed since the last time it was invoked.
 * 
 * @param fn - The function to debounce
 * @param delay - The delay in milliseconds
 * @returns A debounced version of the function with cancel and flush methods
 */
export function debounce<T extends (...args: any[]) => any>(
	fn: T,
	delay: number
): {
	(...args: Parameters<T>): void;
	cancel: () => void;
	flush: () => void;
} {
	let timeoutId: ReturnType<typeof setTimeout> | null = null;
	let lastArgs: Parameters<T> | null = null;

	const debounced = (...args: Parameters<T>) => {
		lastArgs = args;
		
		// Clear existing timeout
		if (timeoutId !== null) {
			clearTimeout(timeoutId);
		}

		// Set new timeout
		timeoutId = setTimeout(() => {
			if (lastArgs) {
				fn(...lastArgs);
				lastArgs = null;
			}
			timeoutId = null;
		}, delay);
	};

	// Cancel any pending execution
	debounced.cancel = () => {
		if (timeoutId !== null) {
			clearTimeout(timeoutId);
			timeoutId = null;
		}
		lastArgs = null;
	};

	// Execute immediately with last args if pending
	debounced.flush = () => {
		if (timeoutId !== null) {
			clearTimeout(timeoutId);
			timeoutId = null;
		}
		if (lastArgs) {
			fn(...lastArgs);
			lastArgs = null;
		}
	};

	return debounced;
}

/**
 * Creates a throttled version of a function that only invokes the function at most once
 * per specified time period.
 * 
 * @param fn - The function to throttle
 * @param limit - The time limit in milliseconds
 * @returns A throttled version of the function
 */
export function throttle<T extends (...args: any[]) => any>(
	fn: T,
	limit: number
): (...args: Parameters<T>) => void {
	let inThrottle = false;
	let lastArgs: Parameters<T> | null = null;

	return (...args: Parameters<T>) => {
		if (!inThrottle) {
			fn(...args);
			inThrottle = true;
			
			setTimeout(() => {
				inThrottle = false;
				if (lastArgs) {
					fn(...lastArgs);
					lastArgs = null;
				}
			}, limit);
		} else {
			lastArgs = args;
		}
	};
}
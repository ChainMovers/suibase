// Class acting as read-only containers.
//
// Intended to be used as stored values for svelte stores.
//
// Can be initialized only from constructor.

import type { ILoadedState, IContextKeyed } from "./poc-interfaces";

export class POCConstructed {
  toString(): string {
    return Object.prototype.toString.call(this) + " = " + JSON.stringify(this);
  }
}

/*
export class EpochLeaderboardHeader extends POCConstructed implements
    IEpochTimeTargets, ILoadedState, IContextKeyed {
    get [Symbol.toStringTag]() {
        return 'EpochLeaderboardHeader';
    }

    readonly isLoaded: boolean = false; // Becomes true if successfully initialized.
    
    readonly e: number; // Epoch.
    readonly remaining: string;
    readonly elapsed: string;
    readonly context_key: string;

    // These are for debug purpose.
    readonly tick: number;
    readonly current: number; // Unix UTC now on last update.

    // Make sure to update isEqual if adding new property here!!!
    constructor( p_e: number, p_remaining: string, p_elapsed: string, p_tick: number, p_current: number, p_context_key: string) {
        super();
        // For IEpochTimeTargets.
        this.e = p_e;
        this.remaining = p_remaining;
        this.elapsed = p_elapsed;

        // The context this object belongs to.
        this.context_key = p_context_key;

        // More values for debugging purpose.
        this.tick = p_tick;
        this.current = p_current
        // Success.
        this.isLoaded = true;        
    }

    isEqual(other: EpochLeaderboardHeader): boolean {
        // Purposely do not compare base properties.
        // Compare only what affects the logic (not the debug values)
        return (
            this.isLoaded == other.isLoaded &&
            this.e == other.e &&
            this.remaining == other.remaining &&
            this.elapsed == other.elapsed
        );
    }
}*/

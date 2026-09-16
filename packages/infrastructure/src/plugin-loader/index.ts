/**
 * Plugin loader interface and basic implementation.
 * Loads capability packages (channels, resources, skills) by ID.
 */

export interface IPluginLoader {
  /**
   * Load a capability instance by ID.
   * ID format: "channel:xingyun", "resource:terminal", "skill:gen-code"
   */
  load<T>(id: string): Promise<T>;

  /**
   * Register a capability instance manually (for testing or bootstrap).
   */
  register(id: string, instance: unknown): void;

  /**
   * Check if a capability is registered.
   */
  has(id: string): boolean;
}

/**
 * Simple in-memory plugin loader.
 * Capabilities are registered manually (no filesystem scanning in MVP).
 */
export class PluginLoader implements IPluginLoader {
  private readonly registry = new Map<string, unknown>();

  async load<T>(id: string): Promise<T> {
    const instance = this.registry.get(id);
    if (!instance) {
      throw new Error(`Plugin not found: ${id}. Available: [${[...this.registry.keys()].join(", ")}]`);
    }
    return instance as T;
  }

  register(id: string, instance: unknown): void {
    this.registry.set(id, instance);
  }

  has(id: string): boolean {
    return this.registry.has(id);
  }
}

export enum HealthState {
    Healthy = 'healthy',
    Unhealthy = 'unhealthy',
    Degraded = 'degraded',
    Unknown = 'unknown',
}

export interface ComponentHealth {
    status: HealthState;
    message?: string;
    [k: string]: unknown;
}

export interface HealthComponents {
    storage: ComponentHealth;
    postgres: ComponentHealth;
    rpc: ComponentHealth;
    [k: string]: ComponentHealth;
}

export interface HealthStatus {
    status: HealthState;
    version: string;
    service: string;
    components: HealthComponents;
    // Allow future changes in response without breaking the type
    [k: string]: unknown;
}

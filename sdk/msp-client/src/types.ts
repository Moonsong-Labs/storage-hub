export enum HealthState {
    Healthy = 'healthy',
    Unhealthy = 'unhealthy',
    Degraded = 'degraded',
    Unknown = 'unknown',
}

export interface HealthComponents {
    storage: HealthState;
    postgres: HealthState;
    rpc: HealthState;
    // Allow future changes in response without breaking the type
    [k: string]: HealthState;
}

export interface HealthStatus {
    status: HealthState;
    version: string;
    service: string;
    components: HealthComponents;
    [k: string]: unknown;
}

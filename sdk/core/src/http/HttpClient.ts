import { HttpError, NetworkError, TimeoutError } from './errors';

export type HttpClientConfig = {
    baseUrl: string;
    timeoutMs?: number;
    defaultHeaders?: Record<string, string>;
    fetchImpl?: typeof fetch;
};

export type RequestOptions = {
    headers?: Record<string, string>;
    signal?: AbortSignal;
    query?: Record<string, string | number | boolean>;
};

export class HttpClient {
    private readonly baseUrl: string;
    private readonly timeoutMs?: number;
    private readonly defaultHeaders: Record<string, string>;
    private readonly fetchImpl: typeof fetch;

    constructor(options: HttpClientConfig) {
        if (!options.baseUrl) throw new Error('HttpClient: baseUrl is required');
        this.baseUrl = options.baseUrl.replace(/\/$/, '');
        this.timeoutMs = options.timeoutMs;
        this.defaultHeaders = { Accept: 'application/json', ...(options.defaultHeaders ?? {}) };
        this.fetchImpl = options.fetchImpl ?? fetch;
    }

    async request<T>(method: 'GET' | 'POST' | 'PUT' | 'DELETE', path: string, options: RequestOptions = {}): Promise<T> {
        const url = this.buildUrl(path, options.query);
        const headers = { ...this.defaultHeaders, ...(options.headers ?? {}) };

        // Support timeout via AbortController if no external signal provided
        const controller = !options.signal && this.timeoutMs ? new AbortController() : undefined;
        const signal = options.signal ?? controller?.signal;

        const timer = controller && this.timeoutMs ? setTimeout(() => controller.abort(), this.timeoutMs) : undefined;

        try {
            const res = await this.fetchImpl(url, {
                method,
                headers,
                signal,
            });

            const text = await res.text();
            const maybeJson = this.parseJsonSafely(text);

            if (!res.ok) {
                throw new HttpError(`HTTP ${res.status} for ${method} ${url}`, res.status, maybeJson ?? text);
            }

            return (maybeJson as T) ?? (text as unknown as T);
        } catch (err: any) {
            if (err?.name === 'AbortError') {
                throw new TimeoutError(`Request timed out for ${method} ${path}`);
            }
            if (err instanceof HttpError) throw err;
            throw new NetworkError(err?.message ?? `Network error for ${method} ${path}`);
        } finally {
            if (timer) clearTimeout(timer);
        }
    }

    get<T>(path: string, options?: RequestOptions): Promise<T> {
        return this.request<T>('GET', path, options);
    }

    private buildUrl(path: string, query?: Record<string, string | number | boolean>): string {
        const normalizedPath = path.startsWith('/') ? path : `/${path}`;
        const url = new URL(this.baseUrl + normalizedPath);

        if (query) {
            for (const [k, v] of Object.entries(query)) {
                url.searchParams.set(k, String(v));
            }
        }

        return url.toString();
    }

    private parseJsonSafely(text: string): unknown | undefined {
        if (!text) return undefined;
        try {
            return JSON.parse(text);
        } catch {
            return undefined;
        }
    }
}

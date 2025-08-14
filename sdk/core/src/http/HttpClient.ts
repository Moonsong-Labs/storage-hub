import { HttpError, NetworkError, TimeoutError } from './errors';
const DEFAULT_TIMEOUT_MS = 30_000;

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
    /**
     * Optional request body. If a non-BodyInit object is provided and no
     * explicit Content-Type header is set, it will be JSON-encoded with
     * 'application/json'.
     */
    body?: BodyInit | unknown;
};

export class HttpClient {
    private readonly baseUrl: string;
    private readonly timeoutMs: number;
    private readonly defaultHeaders: Record<string, string>;
    private readonly fetchImpl: typeof fetch;

    constructor(options: HttpClientConfig) {
        if (!options.baseUrl) throw new Error('HttpClient: baseUrl is required');
        this.baseUrl = options.baseUrl.replace(/\/$/, '');
        this.timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
        this.defaultHeaders = { Accept: 'application/json', ...(options.defaultHeaders ?? {}) };
        this.fetchImpl = options.fetchImpl ?? fetch;
    }

    async request<T>(method: 'GET' | 'POST' | 'PUT' | 'DELETE', path: string, options: RequestOptions = {}): Promise<T> {
        const url = this.buildUrl(path, options.query);
        const headers = { ...this.defaultHeaders, ...(options.headers ?? {}) };

        // Support timeout via AbortController if no external signal provided
        const controller = !options.signal && this.timeoutMs > 0 ? new AbortController() : undefined;
        const signal = options.signal ?? controller?.signal;

        const timer: ReturnType<typeof setTimeout> | undefined =
            controller ? setTimeout(() => controller.abort(), this.timeoutMs) : undefined;

        try {
            // Auto-encode JSON bodies if caller passed a plain object and no Content-Type
            let body = options.body as any;
            const hasExplicitContentType = Object.keys(headers).some(
                (h) => h.toLowerCase() === 'content-type'
            );
            const isBodyInitLike =
                typeof body === 'string' ||
                body instanceof Uint8Array ||
                (typeof ArrayBuffer !== 'undefined' && body instanceof ArrayBuffer) ||
                (typeof Blob !== 'undefined' && body instanceof Blob) ||
                (typeof FormData !== 'undefined' && body instanceof FormData) ||
                (typeof ReadableStream !== 'undefined' && body instanceof ReadableStream);

            if (body !== undefined && body !== null && !isBodyInitLike && !hasExplicitContentType) {
                headers['Content-Type'] = 'application/json';
                body = JSON.stringify(body);
            }

            const res = await this.fetchImpl(url, {
                method,
                headers,
                signal: (signal ?? null) as AbortSignal | null,
                body,
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

    /**
     * Perform a request and return the raw Response without reading the body.
     * Useful for binary downloads or streaming.
     */
    async requestRaw(method: 'GET' | 'POST' | 'PUT' | 'DELETE', path: string, options: RequestOptions = {}): Promise<Response> {
        const url = this.buildUrl(path, options.query);
        const headers = { ...this.defaultHeaders, ...(options.headers ?? {}) };

        const controller = !options.signal && this.timeoutMs > 0 ? new AbortController() : undefined;
        const signal = options.signal ?? controller?.signal;
        const timer: ReturnType<typeof setTimeout> | undefined =
            controller ? setTimeout(() => controller.abort(), this.timeoutMs) : undefined;

        try {
            let body = options.body as any;
            const hasExplicitContentType = Object.keys(headers).some((h) => h.toLowerCase() === 'content-type');
            const isBodyInitLike =
                typeof body === 'string' ||
                body instanceof Uint8Array ||
                (typeof ArrayBuffer !== 'undefined' && body instanceof ArrayBuffer) ||
                (typeof Blob !== 'undefined' && body instanceof Blob) ||
                (typeof FormData !== 'undefined' && body instanceof FormData) ||
                (typeof ReadableStream !== 'undefined' && body instanceof ReadableStream);

            if (body !== undefined && body !== null && !isBodyInitLike && !hasExplicitContentType) {
                headers['Content-Type'] = 'application/json';
                body = JSON.stringify(body);
            }

            const res = await this.fetchImpl(url, {
                method,
                headers,
                signal: (signal ?? null) as AbortSignal | null,
                body,
            });

            if (!res.ok) {
                // Consume error body as text for useful diagnostics
                const text = await res.text();
                const maybeJson = this.parseJsonSafely(text);
                throw new HttpError(`HTTP ${res.status} for ${method} ${url}`, res.status, maybeJson ?? text);
            }

            return res;
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

    post<T>(path: string, options?: RequestOptions): Promise<T> {
        return this.request<T>('POST', path, options ?? {});
    }

    put<T>(path: string, options?: RequestOptions): Promise<T> {
        return this.request<T>('PUT', path, options ?? {});
    }

    delete<T>(path: string, options?: RequestOptions): Promise<T> {
        return this.request<T>('DELETE', path, options ?? {});
    }

    getRaw(path: string, options?: RequestOptions): Promise<Response> {
        return this.requestRaw('GET', path, options ?? {});
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

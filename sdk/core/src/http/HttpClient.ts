import { HttpError, NetworkError, TimeoutError } from './errors.js';
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
  /**
   * If true, returns the raw Response without consuming the body.
   * Useful for streaming downloads.
   */
  raw?: boolean;
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

  async request<T>(
    method: 'GET' | 'POST' | 'PUT' | 'DELETE',
    path: string,
    options: RequestOptions = {},
  ): Promise<T | Response> {
    const url = this.buildUrl(path, options.query);
    const headers = { ...this.defaultHeaders, ...(options.headers ?? {}) };

    // Support timeout via AbortController if no external signal provided
    const controller = !options.signal && this.timeoutMs > 0 ? new AbortController() : undefined;
    const signal = options.signal ?? controller?.signal;

    const timer: ReturnType<typeof setTimeout> | undefined = controller
      ? setTimeout(() => controller.abort(), this.timeoutMs)
      : undefined;

    try {
      // Auto-encode JSON bodies if caller passed a plain object and no Content-Type
      const hasExplicitContentType = Object.keys(headers).some(
        (h) => h.toLowerCase() === 'content-type',
      );
      const candidate = options.body;
      let body: BodyInit | null = null;
      if (candidate !== undefined && candidate !== null) {
        if (this.isBodyInit(candidate)) {
          body = candidate;
        } else {
          // For non-BodyInit payloads, send JSON
          if (!hasExplicitContentType) headers['Content-Type'] = 'application/json';
          body = JSON.stringify(candidate);
        }
      }

      const init: RequestInit = {
        method,
        headers,
        ...(signal ? { signal } : {}),
        ...(body !== null ? { body } : {}),
      };
      const fetchFn =
        typeof globalThis !== 'undefined' &&
        this.fetchImpl === (globalThis as unknown as { fetch: typeof fetch }).fetch
          ? (globalThis as unknown as { fetch: typeof fetch }).fetch.bind(globalThis)
          : this.fetchImpl;
      const res = await fetchFn(url, init);

      // If the response is not OK, consume body for error details and throw
      if (!res.ok) {
        const text = await res.text();
        const maybeJson = this.parseJsonSafely(text);
        throw new HttpError(
          `HTTP ${res.status} for ${method} ${url}`,
          res.status,
          maybeJson ?? text,
        );
      }

      // If raw response requested, return it without consuming the body
      if (options.raw) {
        return res as Response;
      }

      // Normal response processing - consume and parse the body
      const text = await res.text();
      const maybeJson = this.parseJsonSafely(text);
      return (maybeJson as T) ?? (text as unknown as T);
    } catch (err: unknown) {
      if (this.isAbortError(err)) {
        throw new TimeoutError(`Request timed out for ${method} ${path}`);
      }
      if (err instanceof HttpError) throw err;
      const msg = this.getErrorMessage(err);
      throw new NetworkError(msg ?? `Network error for ${method} ${path}`);
    } finally {
      if (timer) clearTimeout(timer);
    }
  }

  get<T>(path: string, options?: RequestOptions): Promise<T> {
    return this.request<T>('GET', path, options) as Promise<T>;
  }

  post<T>(path: string, options?: RequestOptions): Promise<T> {
    return this.request<T>('POST', path, options ?? {}) as Promise<T>;
  }

  put<T>(path: string, options?: RequestOptions): Promise<T> {
    return this.request<T>('PUT', path, options ?? {}) as Promise<T>;
  }

  delete<T>(path: string, options?: RequestOptions): Promise<T> {
    return this.request<T>('DELETE', path, options ?? {}) as Promise<T>;
  }

  getRaw(path: string, options?: RequestOptions): Promise<Response> {
    return this.request('GET', path, { ...options, raw: true }) as Promise<Response>;
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

  private isBodyInit(value: unknown): value is BodyInit {
    return (
      typeof value === 'string' ||
      value instanceof Uint8Array ||
      (typeof ArrayBuffer !== 'undefined' && value instanceof ArrayBuffer) ||
      (typeof Blob !== 'undefined' && value instanceof Blob) ||
      (typeof FormData !== 'undefined' && value instanceof FormData) ||
      (typeof ReadableStream !== 'undefined' && value instanceof ReadableStream)
    );
  }

  private isAbortError(err: unknown): err is { name: string } {
    return (
      typeof err === 'object' &&
      err !== null &&
      'name' in err &&
      typeof (err as { name?: unknown }).name === 'string' &&
      (err as { name: string }).name === 'AbortError'
    );
  }

  private getErrorMessage(err: unknown): string | undefined {
    if (typeof err === 'string') return err;
    if (typeof err === 'object' && err !== null && 'message' in err) {
      const m = (err as { message?: unknown }).message;
      if (typeof m === 'string') return m;
    }
    return undefined;
  }
}

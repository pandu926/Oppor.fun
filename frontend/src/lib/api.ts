import { apiBase, demo, secureUrl } from "./config";
export class ApiError extends Error {
  constructor(
    public code: string,
    message: string,
    public status: number,
    public requestId?: string,
  ) {
    super(message);
  }
}
let csrf: string | null = null;
const pending = new Map<string, string>();
export function setCsrf(value: string | null) {
  csrf = value;
  pending.clear();
}
export async function request<T>(
  path: string,
  options: {
    method?: string;
    body?: unknown;
    key?: string;
    signal?: AbortSignal;
  } = {},
): Promise<T> {
  if (demo)
    throw new Error(
      "This action requires live mode. Demo funds cannot be transferred.",
    );
  const method = options.method || "GET";
  const unsafe = method !== "GET";
  const headers: Record<string, string> = { Accept: "application/json" };
  if (options.body !== undefined) headers["Content-Type"] = "application/json";
  const fingerprint = JSON.stringify([csrf, path, options.body]);
  if (unsafe) {
    if (csrf) headers["X-CSRF-Token"] = csrf;
    headers["Idempotency-Key"] =
      options.key || pending.get(fingerprint) || crypto.randomUUID();
  }
  const base = apiBase.startsWith("/")
    ? apiBase
    : secureUrl(apiBase, import.meta.env.DEV).replace(/\/$/, "");
  if (unsafe) pending.set(fingerprint, headers["Idempotency-Key"]);
  let response: Response;
  try {
    response = await fetch(base + path, {
      method,
      credentials: "include",
      headers,
      body:
        options.body === undefined ? undefined : JSON.stringify(options.body),
      signal: options.signal || AbortSignal.timeout(20000),
    });
  } catch {
    throw new ApiError(
      "NETWORK_ERROR",
      "Could not reach the server. Check your connection before retrying.",
      0,
    );
  }
  const data = await response.json().catch(() => null);
  if (response.status !== 503 && response.status !== 504)
    pending.delete(fingerprint);
  if (!response.ok)
    throw new ApiError(
      data?.error?.code || "REQUEST_FAILED",
      data?.error?.message || "The request could not be completed.",
      response.status,
      data?.error?.request_id,
    );
  return data as T;
}
export function post<T>(path: string, body?: unknown, key?: string) {
  return request<T>(path, { method: "POST", body, key });
}
export function rememberCampaign(wallet: string, id: string) {
  try {
    const key = `oppor:entries:${wallet.toLowerCase()}`;
    const ids = JSON.parse(localStorage.getItem(key) || "[]") as string[];
    localStorage.setItem(
      key,
      JSON.stringify([...new Set([...ids, id])].slice(-500)),
    );
  } catch {
    /* Optional public IDs only. */
  }
}
export function rememberedCampaigns(wallet: string): string[] {
  try {
    return JSON.parse(
      localStorage.getItem(`oppor:entries:${wallet.toLowerCase()}`) || "[]",
    );
  } catch {
    return [];
  }
}

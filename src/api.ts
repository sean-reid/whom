import type {
  ErrorResponse,
  GuessResponse,
  NamesResponse,
  PuzzleResponse,
  StatsResponse,
} from "../shared/api.ts";

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}

async function call<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    ...init,
    headers: { "content-type": "application/json", ...(init?.headers ?? {}) },
  });
  const body: unknown = await res.json().catch(() => null);
  if (!res.ok) {
    const message = (body as ErrorResponse | null)?.error ?? `HTTP ${res.status}`;
    throw new ApiError(res.status, message);
  }
  return body as T;
}

export const getPuzzle = (date: string): Promise<PuzzleResponse> =>
  call(`/api/puzzle?date=${encodeURIComponent(date)}`);

export const postGuess = (token: string, name: string): Promise<GuessResponse> =>
  call("/api/guess", { method: "POST", body: JSON.stringify({ token, name }) });

export const getNames = (version: string): Promise<NamesResponse> =>
  call(`/api/names?v=${encodeURIComponent(version)}`);

export const getStats = (n: number): Promise<StatsResponse> => call(`/api/stats/${n}`);

import { MAX_GUESSES } from "../shared/api.ts";

export const SITE = "whom.dwainosaur.com";

export function shareLine(n: number, won: boolean, guesses: number): string {
  return `WHOM? #${n} ${won ? guesses : "X"}/${MAX_GUESSES} ${SITE}`;
}

export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

// Tipe minimal untuk tes yang membaca berkas lewat Node (vitest); menghindari dependensi @types/node.
declare module "node:fs" {
  export function existsSync(path: string): boolean;
  export function statSync(path: string): { size: number };
  export function readFileSync(path: string): { subarray(start: number, end: number): { toString(enc: string): string } };
  export function readFileSync(path: string, enc: "utf8"): string;
}
declare module "node:url" {
  export function fileURLToPath(url: URL): string;
}

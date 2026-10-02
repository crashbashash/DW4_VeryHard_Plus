/** One inspection verdict, mirroring the old StatusKind mapping. */
export interface DiscStatus {
  kind: "ok" | "refused" | "unknown";
  text: string;
  size: number;
  copies: [number, number, number] | null;
}

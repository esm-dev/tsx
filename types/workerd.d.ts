export interface Fetcher {
  fetch(req: Request): Promise<Response>;
}
declare const fetcer: Fetcher;
export default fetcer;

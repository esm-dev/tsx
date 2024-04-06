export class TransformService {
  transform: typeof import("./swc").transform;
  transformCSS: typeof import("./lightningcss").transform;
}

export interface Fetcher {
  fetch(req: Request): Promise<Response>;
}
declare const fetcer: Fetcher;
export default fetcer;

import { writable, type Writable, get } from "svelte/store";
import { LSUI_CONTEXT_KEY, VITE_SSC_API_URL } from "../L1/consts";

// Step for a context switch:
//    - Make all context store use the proper context. This will make the UI
//      use the proper sources of data.
//    - Trig loop refresh(). Will properly update all table/graph/data stores in new context.

export const global_srv: Writable<string> = writable("");
export const global_url_proxy: Writable<string> = writable(String("http://0.0.0.0:44399"));
export const global_context: Writable<string> = writable(LSUI_CONTEXT_KEY);

const rsplit = function (source: string, sep: string, maxsplit: number) {
  const split = source.split(sep);
  return maxsplit ? [split.slice(0, -maxsplit).join(sep)].concat(split.slice(-maxsplit)) : split;
};

let _min_headers_key = "";
export const min_headers_key = function (): string {
  if (_min_headers_key == "") {
    _min_headers_key = "Accept-Language";
  }
  return _min_headers_key;
};

let _min_headers_value = "";
export const min_headers_value = function (): string {
  if (_min_headers_value == "") {
    _min_headers_value = "en-US,en;q=0.9,fr;q=0.8";
  }
  return _min_headers_value;
};

export const init_srv = function (srv_str: string) {
  const url_proxy: string = get(global_url_proxy).toString();
  const sbeg = rsplit(url_proxy, "/", 1)[0] + "/";
  global_url_proxy.set(sbeg + srv_str + ".mhax.io");
  global_srv.set(srv_str);
};

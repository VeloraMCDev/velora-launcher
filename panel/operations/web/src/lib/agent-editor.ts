export type Target = {name:string;kind:'minecraft'|'http'|'tcp';host:string;port:number;url:string;virtual_host:string;original?:Record<string,unknown>};
export type HostConfiguration = Record<string,unknown> & {id:string;token:string;ops_url:string;services:Record<string,unknown>[]};
const text = (v:unknown, max:number):v is string => typeof v === 'string' && v.trim().length > 0 && v.length <= max && !/[\x00-\x1f\x7f]/.test(v);
export function serviceTargets(services:unknown):Target[] {
  if (!Array.isArray(services) || services.length > 64) throw Error('Maximum 64 monitored services');
  return services.map(s=>{
    if (!s || typeof s !== 'object' || !['minecraft','tcp','http'].includes(s.kind)) throw Error('Invalid service type');
    return {name:s.name,kind:s.kind,host:s.host??'',port:s.port??25565,url:s.url??'',virtual_host:s.virtual_host??'',original:structuredClone(s)};
  });
}
export function editedServices(targets:Target[]):Record<string,unknown>[] {
  if (targets.length > 64) throw Error('Maximum 64 monitored services');
  const names = new Set();
  return targets.map(t=>{
    if (!text(t.name,80) || names.has(t.name)) throw Error('Service names must be unique and contain 1–80 characters');
    names.add(t.name);
    const s:Record<string,unknown> = {...t.original,name:t.name,kind:t.kind};
    if (t.kind === 'http') {
      let u; try {u=new URL(t.url);} catch {throw Error('Enter a valid HTTP service URL');}
      if (!['http:','https:'].includes(u.protocol) || u.username || u.password) throw Error('Use an HTTP(S) service URL without embedded credentials');
      delete s.host;delete s.port;delete s.virtual_host;s.url=t.url;
    } else {
      if (!['tcp','minecraft'].includes(t.kind) || !text(t.host,253) || !Number.isInteger(Number(t.port)) || Number(t.port)<1 || Number(t.port)>65535
        || (t.virtual_host && !text(t.virtual_host,253))) throw Error('Enter a host and port between 1 and 65535');
      delete s.url;delete s.headers;delete s.virtual_host;s.host=t.host;s.port=Number(t.port);
      if (t.kind === 'minecraft' && t.virtual_host) s.virtual_host=t.virtual_host;
    }
    return s;
  });
}
export function loadHostConfiguration(raw:string, id:string):HostConfiguration {
  if (raw.length > 512*1024) throw Error('Configuration is too large');
  const c=JSON.parse(raw);
  if (!c || c.id !== id) throw Error('Choose the configuration belonging to this agent ID');
  if (!text(c.token,256) || c.token.length<32) throw Error('The host configuration must retain its enrollment token');
  const u=new URL(c.ops_url);
  if (u.protocol !== 'https:' || u.username || u.password || u.search || u.hash) throw Error('The host configuration needs its Ops HTTPS URL');
  const targets=serviceTargets(c.services??[]);editedServices(targets);
  return {...c,services:c.services??[]};
}
export function updateHostConfiguration(c:HostConfiguration, targets:Target[]):HostConfiguration {
  return {...c,services:editedServices(targets)};
}

export class SessionUnavailable extends Error {}
export class ControlClient {
  constructor(fetcher=fetch,onSessionLoss=()=>{}){this.fetcher=fetcher.bind(globalThis);this.onSessionLoss=onSessionLoss;this.epoch=0;}
  async request(path,body){
    if(!/^\/api\/v1\/[a-z0-9/?=&_.%-]+$/.test(path))throw Error('Invalid API path');
    const epoch=this.epoch;
    const init={credentials:'same-origin',redirect:'error',cache:'no-store',signal:AbortSignal.timeout(20000),headers:{accept:'application/json'}};
    if(body!==undefined){init.method='POST';init.headers['content-type']='application/json';init.headers['x-velora-action']='1';init.headers['idempotency-key']=crypto.randomUUID();init.body=JSON.stringify(body);}
    let response;try{response=await this.fetcher(path,init);}catch{throw Error('Connection unavailable. Refresh to try again.');}
    if([401,403].includes(response.status)){this.epoch++;this.onSessionLoss();throw new SessionUnavailable('Session or permission unavailable. Sign in again to continue.');}
    if(epoch!==this.epoch)throw new SessionUnavailable('Session changed. Refresh to continue.');
    if(!response.ok)throw Error(response.status===409?'This action has already changed. Refresh before trying again.':'Request unavailable. Refresh to try again.');
    if(!response.headers.get('content-type')?.includes('application/json')){this.epoch++;this.onSessionLoss();throw new SessionUnavailable('Sign in again to continue.');}
    const result=await response.json();
    if(epoch!==this.epoch)throw new SessionUnavailable('Session changed. Refresh to continue.');
    return result;
  }
}

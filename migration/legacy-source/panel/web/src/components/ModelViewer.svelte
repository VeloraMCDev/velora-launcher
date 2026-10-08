<script lang="ts">
  import { onMount } from 'svelte';
  import { RotateCw, Pause, Play, Grid3x3 } from '@lucide/svelte';
  import type * as THREE from 'three';

  type Face = { uv?: number[]; texture?: string; rotation?: number };
  type Element = { from: number[]; to: number[]; rotation?: { origin: number[]; axis: string; angle: number }; faces?: Record<string, Face> };
  type ViewBundleLike = { mode: string; elements?: unknown[]; textures?: Record<string, string | null>; layers?: string[] };
  export type View = { mode: 'elements'; elements: Element[]; textures: Record<string, string | null> } | { mode: 'flat'; layers: string[] };

  let { view, height = 320, autoRotate = true, scale = 1 }: { view: View | ViewBundleLike | null; height?: number; autoRotate?: boolean; scale?: number } = $props();

  let host: HTMLDivElement | undefined = $state();
  let failed = $state('');
  let spinning = $state(true);
  let grid = $state(true);
  let api: { rebuild: (v: View | null) => void; spin: (on: boolean) => void; showGrid: (on: boolean) => void; reset: () => void; dispose: () => void } | null = null;

  $effect(() => { spinning = autoRotate; });
  // Read the reactive values before touching `api` so the effects always subscribe to them.
  let ready = $state(false);
  $effect(() => { const v = view as View | null; if (ready) api?.rebuild(v); });
  $effect(() => { const on = spinning; if (ready) api?.spin(on); });
  $effect(() => { const on = grid; if (ready) api?.showGrid(on); });

  onMount(() => {
    let disposed = false;
    (async () => {
      try {
        const T = await import('three');
        const { OrbitControls } = await import('three/examples/jsm/controls/OrbitControls.js');
        if (disposed || !host) return;
        const renderer = new T.WebGLRenderer({ antialias: true, alpha: true });
        renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
        host.appendChild(renderer.domElement);
        renderer.domElement.style.display = 'block';
        const scene = new T.Scene();
        const camera = new T.PerspectiveCamera(38, 1, 0.01, 100);
        const controls = new OrbitControls(camera, renderer.domElement);
        controls.enableDamping = true;
        controls.dampingFactor = 0.1;
        controls.minDistance = 0.6;
        controls.maxDistance = 8;
        controls.autoRotate = spinning;
        controls.autoRotateSpeed = 2.4;
        scene.add(new T.AmbientLight(0xffffff, 1.35));
        const sun = new T.DirectionalLight(0xffffff, 1.6);
        sun.position.set(2, 4, 3);
        scene.add(sun);
        const floor = new T.GridHelper(2, 16, 0x5a5f78, 0x2e3140);
        scene.add(floor);
        const group = new T.Group();
        scene.add(group);
        const textures: THREE.Texture[] = [];
        const loader = new T.TextureLoader();

        const makeTexture = (uri: string | null | undefined) => {
          if (!uri) {
            const c = document.createElement('canvas');
            c.width = c.height = 16;
            const g = c.getContext('2d')!;
            g.fillStyle = '#000'; g.fillRect(0, 0, 16, 16);
            g.fillStyle = '#f800f8'; g.fillRect(0, 0, 8, 8); g.fillRect(8, 8, 8, 8);
            const t = new T.CanvasTexture(c);
            t.magFilter = T.NearestFilter; t.minFilter = T.NearestFilter; t.colorSpace = T.SRGBColorSpace;
            textures.push(t);
            return t;
          }
          const t = loader.load(uri);
          t.magFilter = T.NearestFilter; t.minFilter = T.NearestFilter; t.colorSpace = T.SRGBColorSpace; t.flipY = false;
          textures.push(t);
          return t;
        };
        const material = (t: THREE.Texture) => new T.MeshLambertMaterial({ map: t, transparent: true, alphaTest: 0.05, side: T.DoubleSide });

        // Corner order (top-left, top-right, bottom-right, bottom-left) as seen from outside each face.
        const corners = (f: string, a: number[], b: number[]): number[][] => {
          const [x1, y1, z1] = a, [x2, y2, z2] = b;
          switch (f) {
            case 'north': return [[x2, y2, z1], [x1, y2, z1], [x1, y1, z1], [x2, y1, z1]];
            case 'south': return [[x1, y2, z2], [x2, y2, z2], [x2, y1, z2], [x1, y1, z2]];
            case 'east': return [[x2, y2, z2], [x2, y2, z1], [x2, y1, z1], [x2, y1, z2]];
            case 'west': return [[x1, y2, z1], [x1, y2, z2], [x1, y1, z2], [x1, y1, z1]];
            case 'up': return [[x1, y2, z1], [x2, y2, z1], [x2, y2, z2], [x1, y2, z2]];
            default: return [[x1, y1, z2], [x2, y1, z2], [x2, y1, z1], [x1, y1, z1]];
          }
        };
        const defaultUv = (f: string, a: number[], b: number[]) => {
          const [x1, y1, z1] = a, [x2, y2, z2] = b;
          if (f === 'north' || f === 'south') return [x1, 16 - y2, x2, 16 - y1];
          if (f === 'east' || f === 'west') return [z1, 16 - y2, z2, 16 - y1];
          return [x1, z1, x2, z2];
        };

        const clear = () => {
          for (const o of [...group.children]) {
            group.remove(o);
            (o as THREE.Mesh).geometry?.dispose();
            const m = (o as THREE.Mesh).material;
            if (Array.isArray(m)) m.forEach((x) => x.dispose()); else (m as THREE.Material | undefined)?.dispose();
          }
          textures.splice(0).forEach((t) => t.dispose());
        };

        const build = (v: View | null) => {
          clear();
          if (!v) return;
          const S = 1 / 16;
          if (v.mode === 'flat') {
            // A sprite with thickness: front and back planes plus a 1px slab.
            v.layers.forEach((uri, n) => {
              const t = makeTexture(uri);
              t.flipY = true;
              const m = material(t);
              const w = 1, d = 1 / 16;
              const front = new T.Mesh(new T.PlaneGeometry(w, w), m);
              front.position.set(0, w / 2, d / 2 + n * 0.002);
              const back = new T.Mesh(new T.PlaneGeometry(w, w), m);
              back.position.set(0, w / 2, -d / 2 - n * 0.002);
              back.rotation.y = Math.PI;
              group.add(front, back);
              if (n === 0) {
                const slab = new T.Mesh(new T.BoxGeometry(w, w, d), new T.MeshLambertMaterial({ color: 0x777777, transparent: true, opacity: 0.0 }));
                slab.position.set(0, w / 2, 0);
                group.add(slab);
              }
            });
            return;
          }
          const mats = new Map<string, THREE.MeshLambertMaterial>();
          const matFor = (key: string) => {
            if (!mats.has(key)) mats.set(key, material(makeTexture(v.textures[key])));
            return mats.get(key)!;
          };
          for (const el of v.elements) {
            const verts: number[] = [], uvs: number[] = [], idx: number[] = [], groups: { start: number; count: number; key: string }[] = [];
            let vi = 0;
            for (const [fname, face] of Object.entries(el.faces ?? {})) {
              const key = (face.texture ?? '').replace(/^#/, '');
              const c = corners(fname, el.from, el.to);
              const uv = face.uv ?? defaultUv(fname, el.from, el.to);
              let q = [[uv[0], uv[1]], [uv[2], uv[1]], [uv[2], uv[3]], [uv[0], uv[3]]].map(([u, w]) => [u / 16, w / 16]);
              const turns = (((face.rotation ?? 0) % 360) + 360) % 360 / 90;
              for (let t = 0; t < turns; t++) q = [q[3], q[0], q[1], q[2]];
              const start = idx.length;
              c.forEach((p, i) => { verts.push(p[0] * S, p[1] * S, p[2] * S); uvs.push(q[i][0], q[i][1]); });
              idx.push(vi, vi + 1, vi + 2, vi, vi + 2, vi + 3);
              groups.push({ start, count: 6, key });
              vi += 4;
            }
            if (!idx.length) continue;
            const geo = new T.BufferGeometry();
            geo.setAttribute('position', new T.Float32BufferAttribute(verts, 3));
            geo.setAttribute('uv', new T.Float32BufferAttribute(uvs, 2));
            geo.setIndex(idx);
            geo.computeVertexNormals();
            const keys = [...new Set(groups.map((g) => g.key))];
            groups.forEach((g) => geo.addGroup(g.start, g.count, keys.indexOf(g.key)));
            const mesh = new T.Mesh(geo, keys.map(matFor));
            if (el.rotation && el.rotation.angle) {
              const o = el.rotation.origin.map((n) => n * S);
              const pivot = new T.Group();
              pivot.position.set(o[0], o[1], o[2]);
              mesh.position.set(-o[0], -o[1], -o[2]);
              pivot.add(mesh);
              const ang = (el.rotation.angle * Math.PI) / 180;
              if (el.rotation.axis === 'x') pivot.rotation.x = ang; else if (el.rotation.axis === 'y') pivot.rotation.y = ang; else pivot.rotation.z = ang;
              group.add(pivot);
            } else group.add(mesh);
          }
          // Centre on the block footprint so spinning looks right.
          group.position.set(-0.5, 0, -0.5);
        };

        const fit = () => {
          const box = new T.Box3().setFromObject(group);
          const size = box.getSize(new T.Vector3());
          const radius = Math.max(size.x, size.y, size.z, 0.5);
          const center = box.getCenter(new T.Vector3());
          if (!isFinite(radius) || box.isEmpty()) { center.set(0, 0.5, 0); }
          controls.target.copy(center);
          const dist = Math.max(radius * 1.9, 1.4);
          camera.position.set(center.x + dist * 0.55, center.y + dist * 0.45, center.z + dist * 0.85);
          controls.update();
        };

        const resize = () => {
          if (!host) return;
          const w = host.clientWidth, h = host.clientHeight;
          renderer.setSize(w, h);
          camera.aspect = w / Math.max(h, 1);
          camera.updateProjectionMatrix();
        };
        const ro = new ResizeObserver(resize);
        ro.observe(host);
        resize();

        let raf = 0;
        const tick = () => {
          raf = requestAnimationFrame(tick);
          controls.update();
          renderer.render(scene, camera);
        };
        tick();

        api = {
          rebuild: (v) => { build(v); if (group.children.length) { group.scale.setScalar(Math.min(scale, 1)); fit(); } },
          spin: (on) => { controls.autoRotate = on; },
          showGrid: (on) => { floor.visible = on; },
          reset: fit,
          dispose: () => {
            cancelAnimationFrame(raf);
            ro.disconnect();
            clear();
            controls.dispose();
            renderer.dispose();
            renderer.domElement.remove();
          },
        };
        api.rebuild(view as View | null);
        api.showGrid(grid);
        ready = true;
      } catch (e) {
        failed = e instanceof Error ? e.message : String(e);
      }
    })();
    return () => { disposed = true; api?.dispose(); api = null; };
  });
</script>

<div class="mv" style:height="{height}px">
  <div class="canvas" bind:this={host}></div>
  {#if failed}
    <p class="msg">3D preview isn't available in this browser ({failed}).</p>
  {:else if !view}
    <p class="msg">Nothing to show yet</p>
  {/if}
  <div class="ctl">
    <button type="button" title={spinning ? 'Stop spinning' : 'Spin'} aria-label="Toggle spin" onclick={() => (spinning = !spinning)}>{#if spinning}<Pause size={14} />{:else}<Play size={14} />{/if}</button>
    <button type="button" title="Reset view" aria-label="Reset view" onclick={() => api?.reset()}><RotateCw size={14} /></button>
    <button type="button" class:on={grid} title="Floor grid" aria-label="Toggle floor grid" onclick={() => (grid = !grid)}><Grid3x3 size={14} /></button>
  </div>
  <span class="hint">Drag to rotate · scroll to zoom · right-drag to pan</span>
</div>

<style>
  .mv { position: relative; border-radius: 14px; overflow: hidden; background: radial-gradient(120% 90% at 50% 20%, #2a2d44 0%, #15161f 70%); border: 1px solid var(--line-strong); }
  .canvas { position: absolute; inset: 0; }
  .msg { position: absolute; inset: 0; display: grid; place-items: center; color: var(--muted); font-size: 0.85rem; text-align: center; padding: 20px; pointer-events: none; }
  .ctl { position: absolute; top: 10px; right: 10px; display: flex; gap: 4px; padding: 3px; border-radius: 10px; background: #0008; backdrop-filter: blur(6px); }
  .ctl button { display: grid; place-items: center; width: 28px; height: 28px; border-radius: 7px; border: 0; background: transparent; color: #cfd3ea; cursor: pointer; }
  .ctl button:hover { background: #fff2; }
  .ctl button.on { color: var(--accent-2); }
  .hint { position: absolute; left: 12px; bottom: 8px; font-size: 0.68rem; color: #ffffff77; pointer-events: none; }
</style>

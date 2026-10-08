<script lang="ts">
  import { Monitor, Plus, Trash2, Eye, Save, Copy, Palette, Type, Image, BarChart3, Clock, Heart, Sparkles, Move, X } from '@lucide/svelte/icons';
  import { get, post, put, del } from '../lib/api';
  import { toast } from '../lib/toast.svelte';
  import Empty from '../components/Empty.svelte';
  import Modal from '../components/Modal.svelte';

  type WidgetType = 'health' | 'hunger' | 'xp' | 'money' | 'level' | 'stats' | 'quest_progress' | 'custom_text' | 'image';
  
  type Widget = {
    id: number;
    key: string;
    label: string;
    type: WidgetType;
    position: { x: number; y: number };
    size: { width: number; height: number };
    style: {
      backgroundColor?: string;
      textColor?: string;
      fontSize?: number;
      fontWeight?: string;
      borderRadius?: number;
      padding?: number;
      opacity?: number;
    };
    config: Record<string, any>;
    enabled: boolean;
  };

  type Layout = {
    id: number;
    name: string;
    description: string;
    widgets: Widget[];
    is_default: boolean;
  };

  const defaultStyle = () => ({
    backgroundColor: '#1a1a1a',
    textColor: '#ffffff',
    fontSize: 14,
    fontWeight: 'normal',
    borderRadius: 8,
    padding: 10,
    opacity: 0.9,
  });

  function blankWidget(type: WidgetType = 'custom_text'): Widget {
    return {
      id: 0,
      type,
      key: `${type}_${Date.now()}`,
      label: type,
      position: { x: 100, y: 100 },
      size: { width: 200, height: 40 },
      style: defaultStyle(),
      config: {},
      enabled: true,
    };
  }

  let layouts = $state<Layout[]>([]);
  let selectedLayout = $state<Layout | null>(null);
  let loading = $state(false);
  let previewOpen = $state(false);
  let modalOpen = $state(false);
  let widgetDraft = $state<Widget>(blankWidget());
  let editingWidget = $state<Widget | null>(null);

  const widgetTypes: Array<{ type: WidgetType; label: string; icon: any; description: string }> = [
    { type: 'health', label: 'Health Bar', icon: Heart, description: 'Display player health' },
    { type: 'hunger', label: 'Hunger Bar', icon: BarChart3, description: 'Display hunger level' },
    { type: 'xp', label: 'XP Bar', icon: Sparkles, description: 'Show experience progress' },
    { type: 'money', label: 'Balance Display', icon: BarChart3, description: 'Show player balance' },
    { type: 'level', label: 'Level Display', icon: Sparkles, description: 'Show player level' },
    { type: 'stats', label: 'Stats Panel', icon: BarChart3, description: 'Display custom stats' },
    { type: 'quest_progress', label: 'Quest Progress', icon: Clock, description: 'Show active quest progress' },
    { type: 'custom_text', label: 'Custom Text', icon: Type, description: 'Display custom text or variables' },
    { type: 'image', label: 'Image', icon: Image, description: 'Show a custom image or logo' },
  ];

  async function loadLayouts() {
    loading = true;
    try {
      const res = await get<{ layouts: Layout[] }>('/api/admin/companion/layouts');
      layouts = res.layouts || [];
      // Keep pointing at the same layout, but as the freshly loaded copy.
      selectedLayout = layouts.find((l) => l.id === selectedLayout?.id) ?? layouts[0] ?? null;
    } catch (e) {
      toast(String(e), 'error');
    } finally {
      loading = false;
    }
  }

  async function createLayout() {
    const name = prompt('Layout name:');
    if (!name?.trim()) return;

    try {
      const res = await post<Layout>('/api/admin/companion/layouts', {
        name: name.trim(),
        description: '',
        widgets: [],
      });
      layouts.push(res);
      selectedLayout = res;
      toast('Layout created');
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  async function deleteLayout(layout: Layout) {
    if (!confirm(`Delete "${layout.name}"?`)) return;
    
    try {
      await del(`/api/admin/companion/layouts/${layout.id}`);
      layouts = layouts.filter(l => l.id !== layout.id);
      if (selectedLayout?.id === layout.id) {
        selectedLayout = layouts[0] || null;
      }
      toast('Layout deleted');
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  function openWidgetCreator(type: WidgetType) {
    editingWidget = null;
    widgetDraft = { ...blankWidget(type), label: widgetTypes.find((wt) => wt.type === type)?.label || type };
    modalOpen = true;
  }

  function openWidgetEditor(widget: Widget) {
    editingWidget = widget;
    // A deep copy, so cancelling the dialog leaves the canvas untouched.
    widgetDraft = structuredClone($state.snapshot(widget));
    modalOpen = true;
  }

  async function saveWidget() {
    if (!selectedLayout) return;

    const widget: Widget = structuredClone($state.snapshot(widgetDraft));
    const editing = editingWidget;

    try {
      if (editing) {
        // Update existing widget
        const index = selectedLayout.widgets.findIndex(w => w.id === editing.id);
        if (index !== -1) {
          selectedLayout.widgets[index] = widget;
        }
      } else {
        // Add new widget
        widget.id = Date.now();
        selectedLayout.widgets.push(widget);
      }

      await put(`/api/admin/companion/layouts/${selectedLayout.id}`, selectedLayout);
      toast(editing ? 'Widget updated' : 'Widget added');
      modalOpen = false;
      await loadLayouts();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  async function removeWidget(widget: Widget) {
    if (!selectedLayout || !confirm('Remove this widget?')) return;
    
    selectedLayout.widgets = selectedLayout.widgets.filter(w => w.id !== widget.id);
    
    try {
      await put(`/api/admin/companion/layouts/${selectedLayout.id}`, selectedLayout);
      toast('Widget removed');
      await loadLayouts();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  async function setDefaultLayout(layout: Layout) {
    try {
      await post(`/api/admin/companion/layouts/${layout.id}/set-default`, {});
      layouts = layouts.map(l => ({ ...l, is_default: l.id === layout.id }));
      toast('Default layout set');
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  function duplicateLayout(layout: Layout) {
    const name = prompt('New layout name:', `${layout.name} (Copy)`);
    if (!name?.trim()) return;

    post<Layout>('/api/admin/companion/layouts', {
      name: name.trim(),
      description: layout.description,
      widgets: layout.widgets.map(w => ({ ...w, id: Date.now() + Math.random() })),
    })
      .then(res => {
        layouts.push(res);
        selectedLayout = res;
        toast('Layout duplicated');
      })
      .catch(e => toast(String(e), 'error'));
  }

  $effect(() => {
    void loadLayouts();
  });
</script>

<div class="page">
  <header>
    <div>
      <h1><Monitor size={22} /> Companion Overlay Builder</h1>
      <p>Design custom HUD overlays for the companion app.</p>
    </div>
    <button class="primary" onclick={createLayout}><Plus size={16} /> New layout</button>
  </header>

  <div class="builder-container">
    <aside class="layouts-sidebar">
      <h3>Layouts</h3>
      {#if loading && !layouts.length}
        <div class="loading-small"><span class="spin">◌</span></div>
      {:else if !layouts.length}
        <div class="empty-sidebar">
          <Monitor size={20} />
          <small>No layouts yet</small>
        </div>
      {:else}
        <div class="layout-list">
          {#each layouts as layout (layout.id)}
            <div
              class="layout-item"
              class:active={selectedLayout?.id === layout.id}
              onclick={() => (selectedLayout = layout)}
              role="button"
              tabindex="0"
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); selectedLayout = layout; }}}
            >
              <div class="layout-item-content">
                <strong>{layout.name}</strong>
                {#if layout.is_default}
                  <span class="default-badge">Default</span>
                {/if}
                <small>{layout.widgets.length} widget{layout.widgets.length !== 1 ? 's' : ''}</small>
              </div>
              <div class="layout-item-actions">
                <button class="ghost icon tiny" onclick={(e) => { e.stopPropagation(); duplicateLayout(layout); }} title="Duplicate"><Copy size={12} /></button>
                <button class="ghost icon tiny" onclick={(e) => { e.stopPropagation(); deleteLayout(layout); }} title="Delete"><Trash2 size={12} /></button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </aside>

    <main class="builder-main">
      {#if !selectedLayout}
        <Empty icon={Monitor} title="No layout selected" text="Create a layout to get started." />
      {:else}
        <div class="builder-toolbar">
          <div class="toolbar-left">
            <h2>{selectedLayout.name}</h2>
            {#if !selectedLayout.is_default}
              <button class="ghost small" onclick={() => selectedLayout && setDefaultLayout(selectedLayout)}>Set as default</button>
            {/if}
          </div>
          <button class="ghost icon" onclick={() => (previewOpen = true)} title="Preview"><Eye size={16} /></button>
        </div>

        <div class="widget-types">
          <h4>Add Widget</h4>
          <div class="widget-type-grid">
            {#each widgetTypes as wt (wt.type)}
              <button class="widget-type-card" onclick={() => openWidgetCreator(wt.type)}>
                <svelte:component this={wt.icon} size={20} />
                <span>{wt.label}</span>
              </button>
            {/each}
          </div>
        </div>

        <div class="canvas-container">
          <div class="canvas" style="position: relative; width: 800px; height: 600px; background: #0a0a0a; border: 1px solid var(--line); border-radius: 12px; margin: 20px auto;">
            {#each selectedLayout.widgets as widget (widget.id)}
              <div
                class="widget-preview"
                style="
                  position: absolute;
                  left: {widget.position.x}px;
                  top: {widget.position.y}px;
                  width: {widget.size.width}px;
                  height: {widget.size.height}px;
                  background: {widget.style.backgroundColor};
                  color: {widget.style.textColor};
                  font-size: {widget.style.fontSize}px;
                  font-weight: {widget.style.fontWeight};
                  border-radius: {widget.style.borderRadius}px;
                  padding: {widget.style.padding}px;
                  opacity: {widget.style.opacity};
                  display: flex;
                  align-items: center;
                  justify-content: center;
                  cursor: move;
                  border: 2px solid transparent;
                "
                onclick={() => openWidgetEditor(widget)}
              >
                <span>{widget.label}</span>
                <button
                  class="widget-delete"
                  onclick={(e) => { e.stopPropagation(); removeWidget(widget); }}
                  title="Remove"
                >
                  <X size={12} />
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </main>
  </div>
</div>

<Modal bind:open={modalOpen} title={editingWidget ? 'Edit widget' : 'Add widget'} width={40}>
  {#if widgetDraft.type}
    {@const widgetType = widgetTypes.find(wt => wt.type === widgetDraft.type)}
    
    <div class="form">
      <div class="field">
        <label>Label</label>
        <input bind:value={widgetDraft.label} placeholder={widgetType?.label} />
      </div>

      <div class="field-row">
        <div class="field">
          <label>X Position</label>
          <input type="number" bind:value={widgetDraft.position.x} min="0" max="800" />
        </div>
        <div class="field">
          <label>Y Position</label>
          <input type="number" bind:value={widgetDraft.position.y} min="0" max="600" />
        </div>
      </div>

      <div class="field-row">
        <div class="field">
          <label>Width</label>
          <input type="number" bind:value={widgetDraft.size.width} min="50" max="400" />
        </div>
        <div class="field">
          <label>Height</label>
          <input type="number" bind:value={widgetDraft.size.height} min="20" max="300" />
        </div>
      </div>

      <div class="field">
        <label>Background Color</label>
        <input type="color" bind:value={widgetDraft.style.backgroundColor} />
      </div>

      <div class="field">
        <label>Text Color</label>
        <input type="color" bind:value={widgetDraft.style.textColor} />
      </div>

      <div class="field">
        <label>Font Size</label>
        <input type="number" bind:value={widgetDraft.style.fontSize} min="8" max="48" />
      </div>

      <div class="field">
        <label>Opacity</label>
        <input type="range" bind:value={widgetDraft.style.opacity} min="0" max="1" step="0.1" />
        <small>{((widgetDraft.style.opacity ?? 1) * 100).toFixed(0)}%</small>
      </div>
    </div>
  {/if}

  {#snippet footer()}
    <button class="ghost" onclick={() => (modalOpen = false)}>Cancel</button>
    <button class="primary" onclick={saveWidget}><Save size={15} /> Save widget</button>
  {/snippet}
</Modal>

<style>
  .page { padding: 1.6rem 2.2rem; max-width: 1600px; margin: 0 auto; height: calc(100vh - 3.2rem); display: flex; flex-direction: column; }
  header { display: flex; justify-content: space-between; align-items: center; gap: 16px; margin-bottom: 20px; padding-bottom: 16px; border-bottom: 1px solid var(--line); }
  h1 { display: flex; align-items: center; gap: 10px; margin: 0; font-size: 1.4rem; }
  header p { margin: 6px 0 0; color: var(--muted); font-size: .88rem; }

  .builder-container { display: flex; gap: 16px; flex: 1; min-height: 0; }
  
  .layouts-sidebar { width: 260px; flex: none; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; padding: 16px; display: flex; flex-direction: column; gap: 12px; overflow-y: auto; }
  .layouts-sidebar h3 { margin: 0; font-size: .98rem; font-weight: 600; }
  .layout-list { display: flex; flex-direction: column; gap: 6px; }
  .layout-item { display: flex; align-items: center; gap: 8px; padding: 10px; background: var(--bg-2); border: 1px solid var(--line); border-radius: 10px; text-align: left; }
  .layout-item.active { border-color: var(--accent); background: var(--accent-soft); }
  .layout-item-content { flex: 1; display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .layout-item-content strong { font-size: .88rem; font-weight: 600; }
  .layout-item-content small { font-size: .74rem; color: var(--muted); }
  .default-badge { padding: 2px 5px; border-radius: 4px; background: var(--good)20; color: var(--good); font-size: .7rem; font-weight: 600; text-transform: uppercase; }
  .layout-item-actions { display: flex; gap: 2px; }

  .builder-main { flex: 1; display: flex; flex-direction: column; gap: 16px; overflow-y: auto; }
  .builder-toolbar { display: flex; justify-content: space-between; align-items: center; padding: 14px 18px; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; }
  .toolbar-left { display: flex; align-items: center; gap: 12px; }
  .toolbar-left h2 { margin: 0; font-size: 1.1rem; font-weight: 600; }

  .widget-types { padding: 14px 18px; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; }
  .widget-types h4 { margin: 0 0 10px; font-size: .92rem; font-weight: 600; }
  .widget-type-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); gap: 8px; }
  .widget-type-card { display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 12px; background: var(--bg-2); border: 1px solid var(--line); border-radius: 10px; font-size: .82rem; }
  .widget-type-card:hover { border-color: var(--accent); }

  .canvas-container { padding: 20px; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; }
  .widget-preview { position: relative; transition: border-color .15s; }
  .widget-preview:hover { border-color: var(--accent) !important; }
  .widget-delete { position: absolute; top: 2px; right: 2px; width: 20px; height: 20px; border-radius: 5px; background: var(--bad); color: white; border: none; display: grid; place-items: center; opacity: 0; transition: opacity .15s; cursor: pointer; }
  .widget-preview:hover .widget-delete { opacity: 1; }

  .empty-sidebar { display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 30px 10px; color: var(--muted); }
  .loading-small { display: flex; justify-content: center; padding: 20px; color: var(--muted); }
  .spin { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .form { display: flex; flex-direction: column; gap: 12px; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field label { font-size: .88rem; font-weight: 550; color: var(--text-2); }
  .field-row { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .field small { font-size: .8rem; color: var(--muted); }
</style>

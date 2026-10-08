import type { Component } from 'svelte';
import type { Experience } from './index';
import FrontiersOverview from './FrontiersOverview.svelte';

/** Install experience components here once; panel and launcher use the same registry.
 * Manifest configuration selects a component ID, never executable remote code.
 */
export const widgetComponents: Record<string, Component<{
  experience: Experience;
  widget: Experience['widgets'][number];
}>> = {
  frontiers: FrontiersOverview,
};

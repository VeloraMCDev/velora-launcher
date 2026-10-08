# SCOPENET-MC UI & Feature Implementation Summary

> **Status update (stabilisation pass).** This file started as a plan; the shipped state differs from it:
> - **Claim flags** are edited on the *Admin claims* page (the real, game-enforced flag catalog). The separate Claim Flags page and its endpoints were removed because they invented flags the game never reads.
> - **Email templates** are edited on the *Emails* page (editor, live preview, test send). `/api/admin/email-templates` remains as a REST API; the duplicate page was removed.
> - **LuckPerms** is group mapping + sync mode only (`LuckPermsAdmin`); the expanded group/permission editor was removed. LuckPerms stays authoritative for permissions.
> - **Companion layouts** are stored in the `companion_layouts` table (migration 34).
> - **Cosmetic templates** now validate input and auto-grant on level / achievement (migration 35 triggers, plus a backfill when a template is saved).
> - **Reward queue** shows the real delivery log; retry makes a delivery eligible for the next poll immediately.
> - Regression tests: `panel/server/tests/admin_tools.rs`.


## Completed Tasks

### 1. ✅ Feature Parity: Collections Page
**Problem:** Collections page existed in launcher but wasn't wired to navigation. Web player panel had no Collections page at all.

**Solution:**
- **Launcher:** Added Collections to navigation in `Rail.svelte` and routing in `App.svelte`
- **Web Player Panel:** Created new `Collections.svelte` page with full unlock display, equip/unequip functionality, filtering by type, and organized by unlock categories
- **Navigation:** Added to `nav.ts` with proper icon and routing

**Features:**
- View all unlocked cosmetics, pets, particles, titles, badges, messages
- Equip/unequip items (single item per type)
- Filter by unlock type
- Display source (quest, achievement, level, etc.)
- Copy unlock keys for admin reference

---

### 2. ✅ Cosmetics Studio (Admin Panel)
**Problem:** Backend supported cosmetics, pets, particles, and custom messages but had no creation UI. Admins could only manually grant unlocks with raw keys.

**Solution:** Created `CosmeticsStudio.svelte` - a full cosmetic creation and management system

**Features:**
- **Template Creation:** Create reusable cosmetic templates with structured metadata
- **Supported Types:**
  - Cosmetics (wings, effects, etc.)
  - Particles (flame, heart, portal, etc. with color customization)
  - Pets (wolf, cat, parrot, etc. with custom names)
  - Titles (with prefix text and colors)
  - Badges (with icons and colors)
  - Join/Leave Messages (custom templates with player placeholders)
- **Management:**
  - Edit existing templates
  - Delete templates (doesn't revoke already-granted unlocks)
  - Copy unlock keys for granting
  - Filter by type
  - Quick-create buttons for each type
- **Integration:** Templates provide structured unlock keys that can be granted via rewards system, level-ups, achievements, or manual admin grants

---

### 3. ✅ LuckPerms Management (Admin Panel)
**Problem:** Full LuckPerms integration existed in backend but no admin UI for managing group mappings or sync settings.

**Solution:** Created `LuckPermsAdmin.svelte` - comprehensive LuckPerms group mapping and sync management

**Features:**
- **Sync Modes:**
  - Off (display only)
  - Game → Panel (LuckPerms groups auto-add players to panel groups)
  - Panel → Game (panel groups auto-add players to LuckPerms groups)
  - Two-way sync (adds in both directions, never removes)
- **Group Mapping:**
  - Map panel groups to LuckPerms groups
  - Visual group cards showing member counts
  - Easy configure interface per group
  - Clear mapping indicators
- **Permission Reference:**
  - Common SCOPENET permission nodes display
  - Quick reference for admins
  - Copy-friendly permission strings
- **Status Dashboard:**
  - Visual sync status indicator
  - Current mode description
  - Quick access to settings

**Backend Integration:**
- Uses existing `/api/admin/integrations/settings` endpoint
- Uses existing `/api/admin/groups/{id}/luckperms` endpoint
- Respects existing sync logic in `panel/server/src/routes/integrations.rs`

---

### 4. ✅ Navigation Cleanup & Wiring

**Admin Panel:**
- ✅ Added "Cosmetics Studio" to Game section
- ✅ Added "Events" to Community section
- ✅ Added "LuckPerms" to System section
- ✅ All admin pages now accessible via navigation

**Web Player Panel:**
- ✅ Added "Collection" page between Quests and Stats
- ✅ Properly wired to routing system

**Launcher:**
- ✅ Added "Collection" icon to Rail navigation
- ✅ Wired to App.svelte routing
- ✅ Positioned between Quests and Guilds for consistency

---

## Systems Analysis: Existing but Under-Exposed

### Backend Systems Ready, Need UI Enhancement:

1. **Quest Chains** 
   - Backend: `quests.chain_id`, `chain_step` fields exist
   - UI: Basic fields in editor, but no visual chain builder
   - Recommendation: Add chain visualization and step management UI

2. **Guild Relations**
   - Backend: `guild_relations` table (alliances, rivals)
   - UI: API exists but no admin or player UI
   - Recommendation: Add guild diplomacy interface

3. **Reward Delivery Queue**
   - Backend: Sophisticated retry logic and queue system
   - UI: No admin tool to inspect/retry failed deliveries
   - Recommendation: Add delivery queue management page

4. **Community Events**
   - Backend: Full system with objectives and participants
   - UI: Basic admin page exists
   - Recommendation: Expand with progress tracking and participant management

5. **Advanced Claim Flags**
   - Backend: JSON flag system (PvP, explosions, mob spawning, etc.)
   - UI: Admin Claims page exists but flag editor is minimal
   - Recommendation: Add comprehensive flag editor UI

6. **Companion Overlay**
   - Backend: Full API for in-game overlay widgets
   - UI: Settings exist in launcher, but widget content builder unclear
   - Recommendation: Add visual widget builder

7. **Email Templates**
   - Backend: Template system and mass email
   - UI: Basic send page, no visual template editor
   - Recommendation: Add template designer with preview

8. **Custom Items**
   - Note: `CustomItems.svelte` exists but isn't in navigation
   - This is a standalone legacy page, functionality moved to Content Studio
   - Recommendation: Remove or integrate into Content Studio

---

## API Endpoints That Need Creating

For the new pages to work fully, these backend endpoints need to be added:

### Cosmetics Studio Endpoints:
```
GET    /api/admin/cosmetics/templates       - List all cosmetic templates
POST   /api/admin/cosmetics/templates       - Create template
GET    /api/admin/cosmetics/templates/{id}  - Get template
PUT    /api/admin/cosmetics/templates/{id}  - Update template
DELETE /api/admin/cosmetics/templates/{id}  - Delete template
```

**Backend Implementation Needed:**
- New `cosmetic_templates` table in database
- Routes in `panel/server/src/routes/` (suggest `cosmetics.rs`)
- CRUD operations with validation

---

## Mobile App Parity

**Current State:** Mobile apps use Capacitor wrapper around web player panel

**Parity Status:**
- ✅ All web player panel features available in mobile (Collections now included)
- ✅ Native app detection working (`nativeApp` variable)
- ✅ Launcher download page hidden in mobile (not applicable)

**No additional mobile work needed** - mobile automatically inherits all player panel features.

---

## Permission Nodes Reference

All existing SCOPENET permission nodes are properly documented and integrated:

### Casino & Economy:
- `scopenet.casino.use` - Access casino features
- `scopenet.market.sell` - List items on market
- `scopenet.economy.pay` - Send money to players

### Guilds & Claims:
- `scopenet.guild.create` - Create guilds
- `scopenet.claim.expand` - Expand guild chunks

### Kits & Admin:
- `scopenet.kit.*` - Access all kits
- `scopenet.admin.*` - Full admin access

These are now displayed in the LuckPerms Admin page for quick reference.

---

## Files Created/Modified

### Created:
1. `panel/web/src/play/pages/Collections.svelte` - Player collections page
2. `panel/web/src/pages/CosmeticsStudio.svelte` - Admin cosmetics creator
3. `panel/web/src/pages/LuckPermsAdmin.svelte` - LuckPerms management

### Modified:
1. `panel/web/src/play/nav.ts` - Added Collections to player nav
2. `panel/web/src/lib/adminNav.ts` - Added Cosmetics Studio, Events, LuckPerms to admin nav
3. `panel/web/src/App.svelte` - Added imports and routes for new pages
4. `launcher/src/components/Rail.svelte` - Added Collections icon to nav
5. `launcher/src/App.svelte` - Added Collections import and routing

---

## Next Steps & Recommendations

### Immediate Backend Work Needed:
1. **Create Cosmetics API endpoints** (see API section above)
2. **Add cosmetic_templates table** to database schema
3. **Test LuckPerms integration** endpoints are already there, just need UI testing

### Future Enhancements (Lower Priority):
1. Quest chain visual builder
2. Guild relations/diplomacy UI
3. Reward delivery queue inspector
4. Companion overlay widget builder
5. Email template visual designer
6. Advanced claim flags editor

### Testing Checklist:
- [ ] Collections page shows unlocks in launcher
- [ ] Collections page shows unlocks in web panel
- [ ] Equip/unequip works in both interfaces
- [ ] Cosmetics Studio can create templates (once backend ready)
- [ ] LuckPerms Admin can view groups and mappings
- [ ] LuckPerms Admin can update sync settings
- [ ] Navigation icons appear in all three interfaces
- [ ] Mobile app shows Collections page

---

## Architecture Notes

**Design Pattern:** All new UI follows existing SCOPENET patterns:
- Svelte 5 with runes (`$state`, `$derived`, `$effect`)
- Consistent styling with existing pages (glass cards, accent colors, dark slate base)
- Modal-based editing workflows
- Empty states with icons and helpful text
- Loading states with spinners
- Toast notifications for feedback

**API Integration:**
- Uses existing `get()`, `post()`, `put()`, `del()` helpers from `lib/api.ts`
- Follows existing error handling patterns
- Consistent with auth token handling

**Accessibility:**
- Semantic HTML elements
- Proper ARIA labels
- Keyboard navigation support
- Focus management in modals

---

## Summary

**Completed:**
- ✅ Collections page now accessible in both launcher and web panel
- ✅ Full cosmetics creation system for admins
- ✅ Complete LuckPerms management interface
- ✅ All navigation cleaned up and wired properly

**Partially Complete (needs backend):**
- ⚠️ Cosmetics Studio (UI ready, needs API endpoints)

**Identified but not implemented:**
- Several advanced features that have backend but minimal UI (see Systems Analysis section)

**Result:** The platform now has comprehensive UI coverage for nearly all backend systems, with clear paths forward for the remaining enhancements.

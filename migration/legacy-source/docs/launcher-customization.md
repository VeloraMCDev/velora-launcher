# Launcher customization

In **Global Leveling & Rewards**, edit a title reward and upload its **Rank title PNG**, or enter an HTTPS PNG URL. Keep the reward name: the launcher uses it as accessible alternative text and falls back to it if the image fails. Minecraft chat and permission integrations continue using that text. Images appear in launcher profiles and the quest level summary. The API also supports server title images through `reward_data.title_image`.

The Achievement Creator accepts PNG icons, custom item IDs or emoji, background colors and border colors alongside the existing frame and texture presets. Quest icons accept PNGs, emoji and preset names. Uploads use the existing admin media uploader; panel-relative `/uploads/…` URLs resolve against the connected panel in the launcher. These images customize launcher displays, rather than Minecraft advancement textures.

Home now has a **Hide news / Show news** button. Hiding the feed frees the column for the main launcher content. This uses the existing saved **Show news and announcements** preference in Settings.

The title-bar friends button toggles the right social sidebar. The preference is also available in Settings and survives restarts. The sidebar shows your running instances, friends' online status and current server, searchable friends and the latest five messages on selecting a friend. Previews preserve unread status; **Open conversation** opens the full Social page. Friends refresh every 20 seconds while the sidebar is visible. At smaller window sizes the sidebar overlays the right side and can be closed with its X button.

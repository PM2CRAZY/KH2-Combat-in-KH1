/*
 * kh1kh2_addon.h: the add-on API of KH2 Combat for KH1 (kh1-kh2.dll), version 1.
 * Guide: docs/addons.md. Data (rows, abilities, finishers) goes in the add-on's addon.toml;
 * this API only answers questions at run time.
 *
 * Versioning: version 1 of this table never changes. Anything new comes as version 2 beside it
 * (kh1kh2_addon_api(2)), and version 1 keeps working.
 *
 * Finding it: the mod is "kh1-kh2.dll" when Panacea (OpenKH) loads it from its dll folder, or
 * "dinput8.dll" in KH1's own folder. Look it up with GetModuleHandleA, then
 * GetProcAddress(module, "kh1kh2_addon_api"). Never from your DllMain (the loader's lock is
 * held there and the mod may not be loaded yet): from Panacea's OnInit / OnFrame, a thread of
 * your own, or any later point. Every function may be called from any thread.
 */
#ifndef KH1KH2_ADDON_H
#define KH1KH2_ADDON_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct kh1kh2_addon_api_v1 kh1kh2_addon_api_v1;

/* Called once, on KH1's game thread at the end of a frame, after the add-ons' files are loaded
 * (the first frame end after on_ready was called). Never inside one of KH1's moves. */
typedef void (*kh1kh2_ready_fn)(const kh1kh2_addon_api_v1 *api, void *user);

struct kh1kh2_addon_api_v1 {
    uint32_t size;    /* sizeof(kh1kh2_addon_api_v1) as the mod built it */
    uint32_t version; /* 1 */

    /* Equipped copies of an ability (0 or more); -1 unknown: no save in play, the add-on or the
     * ability isn't loaded, or a bad argument. Up to a second behind the menu. Keys are the
     * addon.toml's [addon] key and [[ability]] key. */
    int32_t (*is_equipped)(const char *addon_key, const char *ability_key);

    /* 1 learned (in its row, equipped or not), 0 not yet; -1 unknown (as is_equipped). */
    int32_t (*is_learned)(const char *addon_key, const char *ability_key);

    /* 1 the add-on is loaded with its row, 0 not (not installed, or refused: see kh1-kh2.log);
     * -1 a bad argument. */
    int32_t (*addon_loaded)(const char *addon_key);

    /* The ability's id in its add-on's own list (its addon.toml `id`, 1..127, the same every
     * run), 0 not loaded; -1 a bad argument. */
    int32_t (*ability_id)(const char *addon_key, const char *ability_key);

    /* Queues cb(api, user) for the next frame end once the add-ons are loaded. 1 queued, 0 not
     * (cb null, or 64 already waiting). */
    int32_t (*on_ready)(kh1kh2_ready_fn cb, void *user);

    /* A line in the mod's log (kh1-kh2.log, written with the debug tools on), as
     * "[add-on <addon_key>] <msg>". UTF-8; at most 2000 lines a session for all add-ons. */
    void (*log)(const char *addon_key, const char *msg);
};

/* The table for `version` (1), or NULL for a version this build of the mod doesn't have. */
typedef const kh1kh2_addon_api_v1 *(*kh1kh2_addon_api_fn)(uint32_t version);
/* Exported by kh1-kh2.dll as: const kh1kh2_addon_api_v1 *kh1kh2_addon_api(uint32_t version); */

#ifdef __cplusplus
}
#endif

#endif /* KH1KH2_ADDON_H */

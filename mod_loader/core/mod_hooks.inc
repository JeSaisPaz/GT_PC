//---------------------------------------------------------------------------------------
// mod_hooks.inc
// Hook point reference — documents ALL locations where ModLoader events fire.
//
// The current design uses event hooks, not function overrides.
// Events are injected at key dispatch points in the MainLoop.
// See mod_loader.ad for the event API.
//---------------------------------------------------------------------------------------

// ─── Current Event Hook Points ────────────────────────────────────────────
//
// These events fire from main_loop_patched.ad's MainLoop:
//
//   onGameStart     - fires once before the first MainLoop iteration
//   beforeMenu      - fires before MGOM.start when entering MENU sequence
//   afterMenu       - fires after MGOM.start exits (menu closed)
//   beforeRace      - fires before GameSequence::executeRace in RACE sequence
//   afterRace       - fires after GamePlan::executeNext in RACE sequence
//   getMenuProject  - QUERY: mods return a string to override which project opens
//   onShutdown      - fires before execShutdown when the application exits
//
// ─── To add new hook points ───────────────────────────────────────────────
//
// 1. In main_loop_patched.ad (or other core script), add at the desired point:
//      ::main::ModLoader::fireEvent("eventName", [arg1, arg2]);
//
// 2. For query events (mod can return a value):
//      var result = ::main::ModLoader::getEventResultString("eventName");
//
// 3. Document the event in this file.
//
// ─── Future Hook Points (planned) ────────────────────────────────────────
//
// These are NOT yet implemented but identified as valuable:
//
//   onRaceStart     - when doStartRace is called (race/ad)
//   onRaceEnd       - when race finishes (result screen)
//   onCarSelect     - when player selects a car (CarRoot)
//   onCourseSelect  - when player selects a course (CourseRoot)
//   onSaveLoad      - when save data is loaded
//   onSaveWrite     - when save data is written
//
// To add them: modify the relevant source file to call fireEvent().

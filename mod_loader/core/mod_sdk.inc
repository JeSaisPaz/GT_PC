//---------------------------------------------------------------------------------------
// mod_sdk.inc
// Mod Development Kit — event-based API for mod authors.
//
// All hooks in the core scripts fire events. Mods register callbacks for events.
// No function override needed — just listen for events and react.
//---------------------------------------------------------------------------------------

// ─── Listen for a game event ─────────────────────────────────────────────
// Callback receives an args array. Multiple mods can listen to the same event.
//
// Events:
//   onGameStart   - Game has booted, first MainLoop iteration
//   beforeMenu    - About to open a menu project
//   afterMenu     - Menu project just opened
//   beforeRace    - About to start a race sequence
//   afterRace     - Race sequence just ended
//
// Queries (return a string):
//   getMenuProject - Override which project to open (e.g., "arcade", "gtmode")
//                    Return the project name string to redirect, or nil for default.
function MOD_ON_EVENT(event_name, callback)
{
    ::main::ModLoader::on(event_name, callback);
}

// ─── Listen for an event once (auto-removes after firing) ────────────────
function MOD_ONCE(event_name, callback)
{
    ::main::ModLoader::once(event_name, callback);
}

// ─── Stop listening for an event ─────────────────────────────────────────
function MOD_OFF(event_name, callback)
{
    ::main::ModLoader::off(event_name, callback);
}

// ─── Fire a custom event (for mod-to-mod communication) ──────────────────
function MOD_FIRE(event_name, args)
{
    ::main::ModLoader::fireEvent(event_name, args);
}

// ─── Player data shortcuts ───────────────────────────────────────────────
function MOD_GET_CASH()
{
    var status = ::main::GlobalStatus::checkout();
    if (status == nil) return 0;
    return status.user_profile.cash;
}

function MOD_SET_CASH(amount)
{
    var status = ::main::GlobalStatus::checkout();
    if (status == nil) return;
    status.user_profile.cash = amount;
}

function MOD_GET_GARAGE_SIZE()
{
    var status = ::main::GlobalStatus::checkout();
    if (status == nil) return 0;
    return status.user_profile.garage.cars.size;
}

function MOD_GIVE_ALL_CARS()
{
    var status = ::main::GlobalStatus::checkout();
    if (status == nil) return;
    var cars = ::main::gtengine::MSpecDB::getCarLabelList();
    foreach (var car in cars)
    {
        var code = ::main::gtengine::MSpecDB::getCarCode(car);
        status.user_profile.garage.addCar(code, 0);
    }
}

// ─── Debug print (uses game's exception log) ─────────────────────────────
function MOD_LOG(message)
{
    try
    {
        ::main::pdiext::MSystemUtility::print(message);
    }
    catch (e) { }
}

// ─── Screen/dialog helpers ───────────────────────────────────────────────
function MOD_SHOW_MESSAGE(context, title, message)
{
    ::main::DialogUtil::openConfirmDialog(
        context,
        ::main::DialogUtil::OK,
        message,
        title
    );
}



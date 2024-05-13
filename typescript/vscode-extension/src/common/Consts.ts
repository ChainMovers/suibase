// Permanent constants
//
// The following should NEVER change because used outside the web app (e.g. backend processing).
//
// Only const of strings and numbers.
//
// No dependency allowed here.

// workdir_idx are hard coded for performance.
// Note: These matches the definition used in the backend.
export const WORKDIR_IDX_MAINNET = 0;
export const WORKDIR_IDX_TESTNET = 1;
export const WORKDIR_IDX_DEVNET = 2;
export const WORKDIR_IDX_LOCALNET = 3;

// List of all possible workdirs planned to be supported.
// The order is important since the position match the WORKDIR_IDX_* constants.
export const WORKDIRS_KEYS = ["mainnet", "testnet", "devnet", "localnet"];
export const WORKDIRS_LABELS = ["Mainnet", "Testnet", "Devnet", "Localnet"];

export const API_URL = "http://0.0.0.0:44399";

// Unique identifier for each participant exchanging messages.
//
// They are used in messages/params when coordinating between views and the extension.
export const WEBVIEW_DASHBOARD = "suibase.dashboard";
export const WEBVIEW_CONSOLE = "suibase.console";
export const WEBVIEW_EXPLORER = "suibase.explorer";
export const WEBVIEW_BACKEND = "suibase.backend"; // Not really a webview, but name similarly for consistency....

// Each Item in a MUI TreeView must have a unique 'id'.
//
// The combination of the following constants with any optional string
// are used to create uniqueness AND context for UI rendering.
//
// The various possible format for a <FolderPath> are:
//    <TopLevel>                : The first level (e.g. "Recent Packages" folder).
//    <FolderType>-<TopLevel>   : The second level (e.g. a Package folder).
//
//    There could be more levels by pre-pending deeper <FolderType>.
//    Some <FolderPath> may have additional string embedded (to help uniqueness).
//
// The format for a unique id:
//    <LeafType>-<FolderPath>-<StringId>
//
// The <StringId> must be unique within the scope of the LeafType and FolderPath.
//
// Examples of tree view id:
//    Recent Packages => "0" for <Top Level>
//    |
//    +-- 0x12~345::demo  => "P-0-IDOPMSR-1234..."} for <FolderType>-<TopLevel>-<StringId>
//      |
//      +-- Init Objects => "I-P-0-IDOPMSR-1234" for <FolderType>-<FolderType>-<TopLevel>-<StringId>
//        |
//        +-- 0x34-567::UpgradeCap => "o-I-P-0-IDOPMSR-1234..." for <LeafType>-<FolderPath>-<StringId>
//
//  "IDOPMSR" is an example of Suibase UUID (defined in the Suibase.yaml for that module instance).
//  "1234..." is an example of unique Sui object ID (without the 0x)

// <TopLevel> are single digit
export const TREE_ITEM_RECENT_PACKAGES = "0";
export const TREE_ITEM_ACCOUNTS = "1";
export const TREE_ITEM_ALL_PACKAGES = "2";

// <FolderType> are uppercase letters.
export const TREE_ITEM_OWNED_COINS = "C";
export const TREE_ITEM_INIT_OBJECTS = "I";
export const TREE_ITEM_OWNED_OBJECTS = "O";
export const TREE_ITEM_PACKAGE = "P";
export const TREE_ITEM_WATCHES = "W";

// <LeafType> are lowercase letters.
export const TREE_ITEM_COIN = "c";
export const TREE_ITEM_ANY_EXPLORER = "e"; // Any string supported by explorer.
export const TREE_ITEM_LOCAL_FILE = "f"; // Will open a local file if clicked.
export const TREE_ITEM_OWNED_OBJECT = "o";
export const TREE_ITEM_STRING = "s"; // Generic string. Like a label (no user action).
export const TREE_ITEM_TIME = "t"; // Unix Epoch Timestamp (will also show "ago").
export const TREE_ITEM_EMPTY = "x"; // Show as (empty) in the UI.

// Some hardcoded id that are handled differently by UI
export const TREE_ITEM_ID_RECENT_PACKAGES_EMPTY = `${TREE_ITEM_EMPTY}-${TREE_ITEM_RECENT_PACKAGES}-help`; // "x-0-help"

// Special string that can be inserted in a Tree View label:
//
//  TREE_ID_INSERT_ADDR
//     Insert in label the last portion of the ID and interpret it as an address.
//     Will shorten the address in the label, add clipboard copy button etc...
export const TREE_ID_INSERT_ADDR = "[TREE_ID_INSERT_ADDR]";

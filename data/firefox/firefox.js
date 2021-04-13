//@ts-check

// ⌥ save to commented YAML

const {assert} = require ('console');
const date = require ('date-and-time');
const fs = require ('fs');
const fsp = fs.promises;
const lz4 = require ('lz4');
const {log, snooze} = require ('log');
const os = require ('os');
const {knuthShuffle} = require ('knuth-shuffle');  // https://stackoverflow.com/a/2450976/257568

class Tab {
  /**
   * @param {string} url
   */
  constructor (url) {
    /** @type {string} */
    this.url = url}}

exports.Tab = Tab

const win = process.platform == 'win32'

/** @returns {Promise<string>} Directory with Firefox profiles */
exports.profiles = async function() {
  const env = process.env['FIREFOX_PROFILES']
  if (fs.existsSync (env)) return env

  // https://support.mozilla.org/en-US/kb/profiles-where-firefox-stores-user-data#w_finding-your-profile-without-opening-firefox
  // “C:\Users\<your Windows login username>\AppData\Roaming\Mozilla\Firefox\Profiles\”
  if (win) {
    const appdata = process.env['APPDATA']
    if (appdata) {
      const profiles = appdata + '\\Mozilla\\Firefox\\Profiles'
      if (fs.existsSync (profiles)) return profiles}}

  throw new Error ('!FIREFOX_PROFILES')}

/**
 * @returns {Promise<Tab[]>}
 */
exports.tabs = async function() {
  const profiles = await exports.profiles()
  const profilesʹ = await fsp.readdir (profiles)
  if (!profilesʹ.length) throw new Error (`No profiles at "${profiles}"`)

  // Sort the profiles by last-modified, in order to pick the recent/actual one
  const lm = /** @type {Map<string, number>} */ (new Map())
  const lmᶠ = /** @param {string} pname */ pname => {
    let ms = lm.get (pname)
    if (!ms) {
      const pstat = fs.statSync (`${profiles}/${pname}`)
      const bexists = fs.existsSync (`${profiles}/${pname}/sessionstore-backups`)
      ms = pstat.isDirectory() && bexists ? pstat.mtimeMs : -1
      lm.set (pname, ms)}
    return ms}
  profilesʹ.sort ((a, b) => lmᶠ (b) - lmᶠ (a))

  const pname = profilesʹ[0]
  const recoveryᵖ = `${profiles}/${pname}/sessionstore-backups/recovery.jsonlz4`
  log (recoveryᵖ)
  const lz4bytes = await fsp.readFile (recoveryᵖ)
  // cf. https://github.com/toashd/mozlz4/blob/d94719e5bae9e9dbd3e64847f91ab61e8646ad8d/index.js#L30
  const magic = 'mozLz40\0'
  assert (lz4bytes.slice (0, 8) .toString ('utf8') == magic)
  const lz4len = lz4bytes.readUInt32LE (8)  // bytes 8..12
  const lz4buf = Buffer.alloc (lz4len)
  const lz4lenʹ = lz4.decodeBlock (lz4bytes, lz4buf, 12)
  assert (lz4lenʹ == lz4len && lz4len == lz4buf.length)
  const lz4str = new TextDecoder ('utf8') .decode (lz4buf)
  const recovery = JSON.parse (lz4str)

  // ⌥ configurable YAML location, env
  const path = os.homedir() + '/.path'
  if (!fs.existsSync (path)) await fsp.mkdir (path, 0o700)
  // NB: Locally we keep the “tabs” separate from the “items” in “path”,
  // for the sake of having a more readable YAML,
  // but the two should also be compatible and synchronizable (a tab is a kind of path item)

  // ⌥ refactor unpacking into a separate function invocable with “node -e”
  //await fsp.writeFile (path + '/fireforx-recovery.json', JSON.stringify (recovery, null, 2))

  const urls = []
  for (const window of recovery.windows) {
    for (const tab of window.tabs) {
      // Every tab has several history entries
      if (!tab.entries || !tab.entries.length) continue
      const entry = tab.entries[tab.entries.length-1]
      urls.push ({url: entry.url, title: entry.title})}}

  const lastUpdate = recovery.session.lastUpdate
  assert (lastUpdate > 1618299928626)
  const lastUpdateˢ = date.format (new Date (lastUpdate), 'YYYY-MM-DD HH:mm', false)
  log (`Found “recovery.json” from ${lastUpdateˢ} with ${urls.length} urls`)
  log (urls)

  return []}

exports.test = async function() {
  const tabs = await exports.tabs()
  assert (tabs && tabs.length)}

function help() {
  console.log ('npm i && node firefox.js --tabs')}

// When invoked from console, “npm i && node firefox.js --tabs”
// cf. https://nodejs.org/dist/latest-v15.x/docs/api/modules.html#modules_accessing_the_main_module
if (require.main === module) (async () => {
  if (process.argv.includes ('--help')) {help(); return}
  if (process.argv.includes ('--tabs')) {
    const tabs = await exports.tabs()
    for (const tab of tabs) {console.log (tab.url)}
    return}

  const shuffleʹ = process.env['STEAM_SHUFFLE']
  const shuffle = shuffleʹ ? parseInt (shuffleʹ) : (process.argv.includes ('--shuffle') ? 9 : 0)
})()

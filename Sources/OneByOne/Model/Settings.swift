import Foundation

/// Where the playlist view sits relative to the track list.
enum PlaylistPos: String, Codable, CaseIterable {
    case top, bottom, right
    var label: String {
        switch self {
        case .top: return "Top"
        case .bottom: return "Bottom"
        case .right: return "Right"
        }
    }
}

/// What the bottom-left panel shows.
enum CoverMode: String, Codable, CaseIterable {
    case artwork, off
    var label: String {
        switch self {
        case .artwork: return "Artwork"
        case .off: return "Off"
        }
    }
}

/// Big view layouts.
enum BigLayout: String, Codable, CaseIterable {
    case titleTimes, titleTimesInfo, titleInfo2, infoTitleTimes
    var label: String {
        switch self {
        case .titleTimes: return "Title, times"
        case .titleTimesInfo: return "Title, times, info"
        case .titleInfo2: return "Title, info, info, times"
        case .infoTitleTimes: return "Title, times, info, info"
        }
    }
}

/// Title bar content.
enum TitleMode: String, Codable, CaseIterable {
    case name, nameElapsed, nameRemaining, nameElapsedTotal
    var label: String {
        switch self {
        case .name: return "Name"
        case .nameElapsed: return "Name and elapsed"
        case .nameRemaining: return "Name and remaining"
        case .nameElapsedTotal: return "Name, elapsed, total"
        }
    }
}

/// Info column content.
enum InfoMode: String, Codable, CaseIterable {
    case bitrate, artistTitle, trackNo, length, bitrateFmt, bitrateTag, yearAlbum, notes
    var label: String {
        switch self {
        case .bitrate: return "Bitrate"
        case .artistTitle: return "Artist - title"
        case .trackNo: return "Track number"
        case .length: return "Length"
        case .bitrateFmt: return "Bitrate and format"
        case .bitrateTag: return "Bitrate and tag"
        case .yearAlbum: return "Year and album"
        case .notes: return "Notes"
        }
    }
}

/// Column the track list is ordered by.
enum SortKey: String, Codable, CaseIterable {
    case name, length, size, type, modified, created
    var label: String {
        switch self {
        case .name: return "Name"
        case .length: return "Length"
        case .size: return "Size"
        case .type: return "Type"
        case .modified: return "Modified"
        case .created: return "Created"
        }
    }
}

/// Colour themes from the Rust build. Each sets the window appearance and list background.
enum Theme: String, Codable, CaseIterable, Equatable {
    case current, dark, anthracite, paper, grey, deepBlue, blue, red
    var label: String {
        switch self {
        case .current: return "Current"
        case .dark: return "Dark mode"
        case .anthracite: return "Anthracite"
        case .paper: return "Paper"
        case .grey: return "Grey"
        case .deepBlue: return "Deep blue"
        case .blue: return "Blue"
        case .red: return "Red"
        }
    }
}

/// Persisted state. JSON in Application Support. Missing keys take the defaults below,
/// so an older file keeps the values it has.
struct Settings: Codable {
    // Session and resume
    var roots: [String] = []
    var lastFolder: String?
    var lastFile: String?
    var lastPos: Double = 0
    /// Folder path to the last file and position played there.
    var folderResume: [String: Resume] = [:]
    var volume: Float = 0.85
    var muted = false
    var repeatMode: Repeat = .off
    var shuffle = false
    var oneTrack = false
    var sort: SortKey = .name
    var sortDesc = false

    // Playback and control
    var autoSearchDirs = false
    var continueFocused = false
    var rememberPositions = true
    var unselectOnChange = false
    var startInBase = false
    var skipListsInSearch = false
    var nextAtEndOfFFwd = true
    var rewindSecs: Int = 5
    var forwardSecs: Int = 5
    var resumeBackMs: Int = 500
    var wheelVolume = true
    var wheelTree = true
    var wheelLists = true
    var fineVolume = false
    var largerSliders = false
    var showTooltips = true
    var tabToolbar = true
    var escQuit = false
    var gapless = true
    var autoSpaceMs: Int = 0
    var fadeStopMs: Int = 0
    var fadeSkipMs: Int = 0
    var goHomeOnChange = false
    var removePlayed = false
    var switchToFolderAtEnd = false
    var reloadAtEnd = false
    var noLabelEdit = false

    // Lists and info
    var restoreList = true
    var fileInfoMenu = true
    var foldersInList = false
    var naturalSort = true
    var lazyTree = true
    var extraInfoCol = true
    var infoMode: InfoMode = .artistTitle
    var noLengthInSize = false
    var alwaysScan = false
    var hideScrollbars = false

    // Window and list style
    var listGrid = false
    var fullRow = true
    var treeLines = true
    var theme: Theme = .current
    var treeSpacing = false
    var treeStyleExplorer = false
    var dimInactiveList: Int = 30
    var biggerTree = false

    // Titles
    var titleMode: TitleMode = .name
    var tagTitles = true
    var upcomingInTitle = false
    var colonMinutes = true

    // Bottom-left panel
    var coverArt = true
    var coverMode: CoverMode = .artwork
    var coverHideEmpty = true
    var coverNames = "folder.jpg;folder.bmp;cover.jpg"

    // Panel sizes
    var treeWidth: Int = 240
    var coverHeight: Int = 160
    var playlistHeight: Int = 160
    var playlistWidth: Int = 300

    // Big view
    var bigView = false
    var bigLayout: BigLayout = .titleTimes
    var bigIdleSecs: Int = 0
    var bigPadding: Int = 12
    var bigCycleSecs: Int = 10
    var bigInfoPattern = "TITLE|INFO|TIME"

    // Playlist
    var playlistPos: PlaylistPos = .bottom
    var playlistOpen = true

    // System
    var oneInstance = true
    var trayIcon = false
    var highPriority = false
    var minimizeOnClose = false
    var autosave5min = true
    var proxy = ""
    var httpEnabled = false
    var httpAddr = "127.0.0.1:8080"

    // Logging and history
    var logEnabled = false
    var logMaxKB: Int = 100
    var historyMenus = false
    var historyEntries: Int = 10
    var recentTracks: [String] = []
    var recentDirs: [String] = []

    // External launchers. `%1` in a template is replaced by the file path.
    var external: [String] = ["", "", ""]

    // Bookmarks
    var bookmarks: [Bookmark] = []

    // Playlist contents, saved across launches
    var playlistEntries: [PlaylistEntry] = []

    // UI session state
    var showTree = true
    var compact = false
    var winWidth: Int = 960
    var winHeight: Int = 620

    struct Resume: Codable, Equatable {
        var path: String
        var seconds: Double
    }

    struct PlaylistEntry: Codable, Equatable {
        var path: String
        /// Cue virtual track bounds. Nil for a whole file.
        var start: Double?
        var end: Double?
    }

    struct Bookmark: Codable, Equatable {
        var title: String
        var path: String
        var isFolder: Bool
    }

    init() {}

    /// Decode each key on its own, so an older file keeps its values and gets defaults for the rest.
    init(from decoder: Decoder) throws {
        self.init()
        let c = try decoder.container(keyedBy: AnyKey.self)
        func get<T: Decodable>(_ key: String, _ into: inout T) {
            if let v = try? c.decodeIfPresent(T.self, forKey: AnyKey(key)) { into = v }
        }
        get("roots", &roots); get("lastFolder", &lastFolder); get("lastFile", &lastFile)
        get("lastPos", &lastPos); get("folderResume", &folderResume); get("volume", &volume)
        get("muted", &muted); get("repeatMode", &repeatMode); get("shuffle", &shuffle)
        get("oneTrack", &oneTrack); get("sort", &sort); get("sortDesc", &sortDesc)
        get("autoSearchDirs", &autoSearchDirs); get("continueFocused", &continueFocused)
        get("rememberPositions", &rememberPositions); get("unselectOnChange", &unselectOnChange)
        get("startInBase", &startInBase); get("skipListsInSearch", &skipListsInSearch)
        get("nextAtEndOfFFwd", &nextAtEndOfFFwd); get("rewindSecs", &rewindSecs)
        get("forwardSecs", &forwardSecs); get("resumeBackMs", &resumeBackMs)
        get("wheelVolume", &wheelVolume); get("wheelTree", &wheelTree); get("wheelLists", &wheelLists)
        get("fineVolume", &fineVolume); get("largerSliders", &largerSliders)
        get("showTooltips", &showTooltips); get("tabToolbar", &tabToolbar); get("escQuit", &escQuit)
        get("gapless", &gapless); get("autoSpaceMs", &autoSpaceMs); get("fadeStopMs", &fadeStopMs)
        get("fadeSkipMs", &fadeSkipMs); get("goHomeOnChange", &goHomeOnChange)
        get("removePlayed", &removePlayed); get("switchToFolderAtEnd", &switchToFolderAtEnd)
        get("reloadAtEnd", &reloadAtEnd); get("noLabelEdit", &noLabelEdit)
        get("restoreList", &restoreList); get("fileInfoMenu", &fileInfoMenu)
        get("foldersInList", &foldersInList); get("naturalSort", &naturalSort)
        get("lazyTree", &lazyTree); get("extraInfoCol", &extraInfoCol); get("infoMode", &infoMode)
        get("noLengthInSize", &noLengthInSize); get("alwaysScan", &alwaysScan)
        get("hideScrollbars", &hideScrollbars)
        get("listGrid", &listGrid); get("fullRow", &fullRow); get("treeLines", &treeLines)
        get("theme", &theme); get("treeSpacing", &treeSpacing); get("treeStyleExplorer", &treeStyleExplorer)
        get("dimInactiveList", &dimInactiveList); get("biggerTree", &biggerTree)
        get("titleMode", &titleMode); get("tagTitles", &tagTitles)
        get("upcomingInTitle", &upcomingInTitle); get("colonMinutes", &colonMinutes)
        get("coverArt", &coverArt); get("coverMode", &coverMode); get("coverHideEmpty", &coverHideEmpty)
        get("coverNames", &coverNames)
        get("treeWidth", &treeWidth); get("coverHeight", &coverHeight)
        get("playlistHeight", &playlistHeight); get("playlistWidth", &playlistWidth)
        get("bigView", &bigView); get("bigLayout", &bigLayout); get("bigIdleSecs", &bigIdleSecs)
        get("bigPadding", &bigPadding); get("bigCycleSecs", &bigCycleSecs)
        get("bigInfoPattern", &bigInfoPattern)
        get("playlistPos", &playlistPos); get("playlistOpen", &playlistOpen)
        get("oneInstance", &oneInstance); get("trayIcon", &trayIcon); get("highPriority", &highPriority)
        get("minimizeOnClose", &minimizeOnClose); get("autosave5min", &autosave5min)
        get("proxy", &proxy); get("httpEnabled", &httpEnabled); get("httpAddr", &httpAddr)
        get("logEnabled", &logEnabled); get("logMaxKB", &logMaxKB); get("historyMenus", &historyMenus)
        get("historyEntries", &historyEntries); get("recentTracks", &recentTracks)
        get("recentDirs", &recentDirs); get("external", &external); get("bookmarks", &bookmarks); get("playlistEntries", &playlistEntries)
        get("showTree", &showTree); get("compact", &compact)
        get("winWidth", &winWidth); get("winHeight", &winHeight)
    }

    static let fileURL: URL = {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("1by1", isDirectory: true)
        try? FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
        return base.appendingPathComponent("settings.json")
    }()

    static func load() -> Settings {
        guard let data = try? Data(contentsOf: fileURL),
              let s = try? JSONDecoder().decode(Settings.self, from: data) else {
            return Settings()
        }
        return s
    }

    func save() {
        let enc = JSONEncoder()
        enc.outputFormatting = [.prettyPrinted, .sortedKeys]
        guard let data = try? enc.encode(self) else { return }
        try? data.write(to: Settings.fileURL, options: .atomic)
    }
}

/// Dynamic coding key for the per-field decoder.
struct AnyKey: CodingKey {
    var stringValue: String
    var intValue: Int? { nil }
    init(_ s: String) { stringValue = s }
    init?(stringValue: String) { self.stringValue = stringValue }
    init?(intValue: Int) { nil }
}

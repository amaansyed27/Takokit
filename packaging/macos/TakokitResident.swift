import AppKit
import Darwin
import Foundation

private let apiBase = "http://127.0.0.1:5050"
private let guiURL = URL(string: "\(apiBase)/gui")!
private let reopenNotification = Notification.Name("com.dawnlightlabs.takokit.open-gui")

private enum ServerState {
    case managed
    case direct
    case stopped
    case unavailable(String)

    var label: String {
        switch self {
        case .managed: return "Running"
        case .direct: return "Running (developer)"
        case .stopped: return "Stopped"
        case .unavailable: return "Unavailable"
        }
    }
}

private final class TakokitAppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate {
    private var statusItem: NSStatusItem?
    private var runtimeRoot: URL?
    private var expectedBuildID = ""
    private var updateVersion: String?
    private var updateCheckRunning = false
    private var explicitLaunch = true
    private var terminating = false

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)
        explicitLaunch = !CommandLine.arguments.contains("--login") && !CommandLine.arguments.contains("--background")

        if anotherInstanceIsRunning() {
            if explicitLaunch {
                DistributedNotificationCenter.default().post(name: reopenNotification, object: nil)
            }
            NSApp.terminate(nil)
            return
        }

        DistributedNotificationCenter.default().addObserver(
            self,
            selector: #selector(handleOpenRequest),
            name: reopenNotification,
            object: nil
        )

        do {
            let root = try resolveRuntimeRoot()
            runtimeRoot = root
            expectedBuildID = readBuildID(root: root)
            try ensureLogDirectory()
            log("resident starting; runtime_root=\(root.path); explicit_launch=\(explicitLaunch)")
            installStatusItem()
            ensureServer(openGUIWhenReady: explicitLaunch)
            checkForUpdates(showResult: false)
        } catch {
            log("startup failure: \(error.localizedDescription)")
            showError("Takokit could not start", detail: error.localizedDescription)
            NSApp.terminate(nil)
        }
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        ensureServer(openGUIWhenReady: true)
        return false
    }

    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        if terminating { return .terminateNow }
        terminating = true
        let state = inspectServer()
        if case .managed = state {
            _ = runTako(["--output", "json", "stop"])
        }
        log("resident quitting; server_state=\(state.label)")
        return .terminateNow
    }

    func menuWillOpen(_ menu: NSMenu) {
        rebuildMenu(menu)
    }

    private func anotherInstanceIsRunning() -> Bool {
        guard let bundleID = Bundle.main.bundleIdentifier else { return false }
        return NSRunningApplication.runningApplications(withBundleIdentifier: bundleID)
            .contains { $0.processIdentifier != ProcessInfo.processInfo.processIdentifier }
    }

    private func installStatusItem() {
        let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        if let iconURL = Bundle.main.url(forResource: "TakokitStatus", withExtension: "png"),
           let image = NSImage(contentsOf: iconURL) {
            image.isTemplate = true
            item.button?.image = image
            item.button?.imagePosition = .imageOnly
        } else {
            item.button?.title = "T"
        }
        item.button?.toolTip = "Takokit \(Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "")"
        let menu = NSMenu(title: "Takokit")
        menu.delegate = self
        item.menu = menu
        statusItem = item
        rebuildMenu(menu)
    }

    private func rebuildMenu(_ menu: NSMenu) {
        menu.removeAllItems()
        let state = inspectServer()
        let status = NSMenuItem(title: "Server: \(state.label)", action: nil, keyEquivalent: "")
        status.isEnabled = false
        menu.addItem(status)
        if case let .unavailable(detail) = state {
            let note = NSMenuItem(title: detail, action: nil, keyEquivalent: "")
            note.isEnabled = false
            menu.addItem(note)
        }
        menu.addItem(.separator())
        addItem(menu, title: "Open GUI", action: #selector(openGUI))
        addItem(menu, title: "Copy API URL", action: #selector(copyAPIURL))
        switch state {
        case .managed:
            addItem(menu, title: "Stop Server", action: #selector(stopServer))
        case .stopped:
            addItem(menu, title: "Start Server", action: #selector(startServer))
        case .direct:
            let item = NSMenuItem(title: "Developer server is not owned by Takokit.app", action: nil, keyEquivalent: "")
            item.isEnabled = false
            menu.addItem(item)
        case .unavailable:
            break
        }
        menu.addItem(.separator())
        if let version = updateVersion {
            addItem(menu, title: "Update to v\(version)", action: #selector(applyUpdate))
        }
        addItem(menu, title: updateCheckRunning ? "Checking for Updates…" : "Check for Updates", action: #selector(checkForUpdatesFromMenu), enabled: !updateCheckRunning)
        addItem(menu, title: "Launch Takokit at Login", action: #selector(toggleLogin), state: loginEnabled() ? .on : .off)
        let version = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "0.3.0"
        addItem(menu, title: "About Takokit v\(version)", action: #selector(showAbout))
        menu.addItem(.separator())
        addItem(menu, title: "Quit Takokit", action: #selector(quit))
    }

    private func addItem(_ menu: NSMenu, title: String, action: Selector, enabled: Bool = true, state: NSControl.StateValue = .off) {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
        item.target = self
        item.isEnabled = enabled
        item.state = state
        menu.addItem(item)
    }

    @objc private func handleOpenRequest() {
        ensureServer(openGUIWhenReady: true)
    }

    @objc private func openGUI() {
        ensureServer(openGUIWhenReady: true)
    }

    @objc private func copyAPIURL() {
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString("\(apiBase)/v1", forType: .string)
    }

    @objc private func startServer() {
        ensureServer(openGUIWhenReady: false)
    }

    @objc private func stopServer() {
        guard case .managed = inspectServer() else { return }
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self else { return }
            let result = self.runTako(["--output", "json", "stop"])
            if result.status != 0 {
                DispatchQueue.main.async {
                    self.showError("Takokit server could not stop", detail: result.stderr.isEmpty ? result.stdout : result.stderr)
                }
            }
        }
    }

    @objc private func checkForUpdatesFromMenu() {
        checkForUpdates(showResult: true)
    }

    @objc private func applyUpdate() {
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self else { return }
            let result = self.runTako(["--output", "json", "update", "apply"])
            if result.status != 0 {
                DispatchQueue.main.async {
                    self.showError("Takokit update failed", detail: result.stderr.isEmpty ? result.stdout : result.stderr)
                }
            }
        }
    }

    @objc private func toggleLogin() {
        do {
            if loginEnabled() {
                try setLoginEnabled(false)
            } else {
                try setLoginEnabled(true)
            }
        } catch {
            showError("Launch at Login could not be changed", detail: error.localizedDescription)
        }
    }

    @objc private func showAbout() {
        let version = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "0.3.0"
        let alert = NSAlert()
        alert.messageText = "Takokit \(version)"
        alert.informativeText = "Local voice AI runtime\n\nGUI: browser-based\nAPI: http://127.0.0.1:5050/v1"
        alert.addButton(withTitle: "OK")
        NSApp.activate(ignoringOtherApps: true)
        alert.runModal()
    }

    @objc private func quit() {
        NSApp.terminate(nil)
    }

    private func ensureServer(openGUIWhenReady: Bool) {
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self else { return }
            var state = self.inspectServer()
            if case .stopped = state {
                let result = self.runTako(["--output", "json", "start"])
                if result.status != 0 {
                    self.log("server start failed: \(result.stderr)")
                    DispatchQueue.main.async {
                        self.showError("Takokit server could not start", detail: result.stderr.isEmpty ? result.stdout : result.stderr)
                    }
                    return
                }
                state = self.waitForServer()
            }
            switch state {
            case .managed, .direct:
                if openGUIWhenReady {
                    DispatchQueue.main.async { NSWorkspace.shared.open(guiURL) }
                }
            case .stopped:
                DispatchQueue.main.async {
                    self.showError("Takokit server did not become ready", detail: "See ~/.takokit/logs for server and resident logs.")
                }
            case let .unavailable(detail):
                DispatchQueue.main.async {
                    self.showError("Takokit server is unavailable", detail: detail)
                }
            }
        }
    }

    private func waitForServer() -> ServerState {
        for _ in 0..<50 {
            let state = inspectServer()
            switch state {
            case .managed, .direct, .unavailable:
                return state
            case .stopped:
                Thread.sleep(forTimeInterval: 0.1)
            }
        }
        return .stopped
    }

    private func inspectServer() -> ServerState {
        guard let root = runtimeRoot else { return .unavailable("Takokit runtime is unresolved") }
        let endpoint = URL(string: "\(apiBase)/api/v1/daemon/identity")!
        var request = URLRequest(url: endpoint)
        request.timeoutInterval = 0.45
        let semaphore = DispatchSemaphore(value: 0)
        var data: Data?
        var response: URLResponse?
        var error: Error?
        URLSession.shared.dataTask(with: request) { body, reply, failure in
            data = body
            response = reply
            error = failure
            semaphore.signal()
        }.resume()
        if semaphore.wait(timeout: .now() + 0.6) == .timedOut {
            return .stopped
        }
        if error != nil { return .stopped }
        guard let http = response as? HTTPURLResponse, http.statusCode == 200, let data else {
            return .unavailable("Port 5050 is occupied by an unverified process")
        }
        guard let value = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let mode = value["mode"] as? String,
              let executable = value["executable"] as? String,
              let buildID = value["build_id"] as? String,
              let host = value["host"] as? String,
              let port = value["port"] as? Int else {
            return .unavailable("Port 5050 returned an invalid Takokit identity")
        }
        let executableParent = URL(fileURLWithPath: executable).standardizedFileURL.deletingLastPathComponent()
        let trustedBin = root.appendingPathComponent("bin", isDirectory: true).standardizedFileURL
        guard executableParent.path == trustedBin.path,
              port == 5050,
              ["127.0.0.1", "localhost", "::1"].contains(host),
              expectedBuildID.isEmpty || buildID == expectedBuildID else {
            return .unavailable("Port 5050 is not the verified Takokit runtime for this installation")
        }
        switch mode {
        case "managed": return .managed
        case "direct": return .direct
        default: return .unavailable("Takokit server published an unknown ownership mode")
        }
    }

    private func checkForUpdates(showResult: Bool) {
        guard !updateCheckRunning else { return }
        updateCheckRunning = true
        DispatchQueue.global(qos: .utility).async { [weak self] in
            guard let self else { return }
            let result = self.runTako(["--output", "json", "update", "check"])
            var available: String?
            var message: String?
            if result.status == 0,
               let data = result.stdout.data(using: .utf8),
               let value = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
                if value["available"] as? Bool == true {
                    available = value["offered_version"] as? String
                }
                message = value["message"] as? String
            } else if result.status != 0 {
                message = result.stderr.isEmpty ? result.stdout : result.stderr
            }
            DispatchQueue.main.async {
                self.updateVersion = available
                self.updateCheckRunning = false
                if showResult {
                    let detail = available.map { "Takokit \($0) is available." }
                        ?? message
                        ?? "No update is currently available."
                    self.showInfo("Takokit Update", detail: detail)
                }
            }
        }
    }

    private func resolveRuntimeRoot() throws -> URL {
        let fm = FileManager.default
        if let explicit = ProcessInfo.processInfo.environment["TAKOKIT_INSTALL_ROOT"], !explicit.isEmpty {
            let root = URL(fileURLWithPath: explicit).standardizedFileURL
            if validRuntimeRoot(root) { return root }
            throw NSError(domain: "Takokit", code: 1, userInfo: [NSLocalizedDescriptionKey: "TAKOKIT_INSTALL_ROOT does not contain bin/tako: \(root.path)"])
        }

        let support = try applicationSupportDirectory()
        let receipt = support.appendingPathComponent("install-root.txt")
        if let text = try? String(contentsOf: receipt, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines), !text.isEmpty {
            let root = URL(fileURLWithPath: text).standardizedFileURL
            if validRuntimeRoot(root) { return root }
        }

        let portable = Bundle.main.bundleURL
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .standardizedFileURL
        if validRuntimeRoot(portable) { return portable }

        let standard = fm.homeDirectoryForCurrentUser
            .appendingPathComponent(".local/share/takokit", isDirectory: true)
            .standardizedFileURL
        if validRuntimeRoot(standard) { return standard }

        throw NSError(domain: "Takokit", code: 2, userInfo: [NSLocalizedDescriptionKey: "Takokit.app could not find its matching runtime. Reinstall Takokit with the supported install.sh path or keep a portable Takokit.app inside its extracted package."])
    }

    private func validRuntimeRoot(_ root: URL) -> Bool {
        FileManager.default.isExecutableFile(atPath: root.appendingPathComponent("bin/tako").path)
            && FileManager.default.fileExists(atPath: root.appendingPathComponent("distribution.json").path)
    }

    private func readBuildID(root: URL) -> String {
        let provenance = root.appendingPathComponent("build-provenance.json")
        guard let data = try? Data(contentsOf: provenance),
              let value = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return "" }
        return value["build_id"] as? String ?? ""
    }

    private func takoURL() -> URL? {
        runtimeRoot?.appendingPathComponent("bin/tako")
    }

    private func runTako(_ arguments: [String]) -> (status: Int32, stdout: String, stderr: String) {
        guard let executable = takoURL() else {
            return (127, "", "Takokit CLI runtime is unresolved")
        }
        let process = Process()
        process.executableURL = executable
        process.arguments = arguments
        let stdout = Pipe()
        let stderr = Pipe()
        process.standardOutput = stdout
        process.standardError = stderr
        do {
            try process.run()
            process.waitUntilExit()
        } catch {
            return (127, "", error.localizedDescription)
        }
        let out = String(data: stdout.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
        let err = String(data: stderr.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
        if process.terminationStatus != 0 { log("tako \(arguments.joined(separator: " ")) failed: \(err)") }
        return (process.terminationStatus, out.trimmingCharacters(in: .whitespacesAndNewlines), err.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    private func applicationSupportDirectory() throws -> URL {
        guard let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first else {
            throw NSError(domain: "Takokit", code: 3, userInfo: [NSLocalizedDescriptionKey: "macOS Application Support directory is unavailable"])
        }
        return base.appendingPathComponent("Takokit", isDirectory: true)
    }

    private func launchAgentURL() -> URL? {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/LaunchAgents/com.dawnlightlabs.takokit.plist")
    }

    private func loginEnabled() -> Bool {
        guard let url = launchAgentURL() else { return false }
        return FileManager.default.fileExists(atPath: url.path)
    }

    private func setLoginEnabled(_ enabled: Bool) throws {
        guard let plistURL = launchAgentURL(), let executable = Bundle.main.executableURL else {
            throw NSError(domain: "Takokit", code: 4, userInfo: [NSLocalizedDescriptionKey: "Takokit.app executable path is unavailable"])
        }
        let fm = FileManager.default
        try fm.createDirectory(at: plistURL.deletingLastPathComponent(), withIntermediateDirectories: true)
        let domain = "gui/\(getuid())"
        if enabled {
            let plist: [String: Any] = [
                "Label": "com.dawnlightlabs.takokit",
                "ProgramArguments": [executable.path, "--login"],
                "RunAtLoad": true,
                "KeepAlive": false
            ]
            let data = try PropertyListSerialization.data(fromPropertyList: plist, format: .xml, options: 0)
            try data.write(to: plistURL, options: .atomic)
            _ = runProcess(URL(fileURLWithPath: "/bin/launchctl"), ["bootout", domain, plistURL.path])
            let result = runProcess(URL(fileURLWithPath: "/bin/launchctl"), ["bootstrap", domain, plistURL.path])
            if result.status != 0 {
                throw NSError(domain: "Takokit", code: 5, userInfo: [NSLocalizedDescriptionKey: result.stderr.isEmpty ? "launchctl bootstrap failed" : result.stderr])
            }
        } else {
            _ = runProcess(URL(fileURLWithPath: "/bin/launchctl"), ["bootout", domain, plistURL.path])
            try? fm.removeItem(at: plistURL)
        }
    }

    private func runProcess(_ executable: URL, _ arguments: [String]) -> (status: Int32, stdout: String, stderr: String) {
        let process = Process()
        process.executableURL = executable
        process.arguments = arguments
        let stdout = Pipe()
        let stderr = Pipe()
        process.standardOutput = stdout
        process.standardError = stderr
        do {
            try process.run()
            process.waitUntilExit()
        } catch {
            return (127, "", error.localizedDescription)
        }
        return (
            process.terminationStatus,
            String(data: stdout.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? "",
            String(data: stderr.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
        )
    }

    private func ensureLogDirectory() throws {
        let dir = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".takokit/logs", isDirectory: true)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    }

    private func log(_ message: String) {
        let url = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".takokit/logs/resident-macos.log")
        let line = "\(ISO8601DateFormatter().string(from: Date())) \(message)\n"
        guard let data = line.data(using: .utf8) else { return }
        if !FileManager.default.fileExists(atPath: url.path) {
            try? data.write(to: url)
            return
        }
        if let handle = try? FileHandle(forWritingTo: url) {
            defer { try? handle.close() }
            try? handle.seekToEnd()
            try? handle.write(contentsOf: data)
        }
    }

    private func showError(_ title: String, detail: String) {
        let alert = NSAlert()
        alert.alertStyle = .critical
        alert.messageText = title
        alert.informativeText = detail.isEmpty ? "See ~/.takokit/logs/resident-macos.log" : detail
        alert.addButton(withTitle: "OK")
        NSApp.activate(ignoringOtherApps: true)
        alert.runModal()
    }

    private func showInfo(_ title: String, detail: String) {
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = detail
        alert.addButton(withTitle: "OK")
        NSApp.activate(ignoringOtherApps: true)
        alert.runModal()
    }
}

let app = NSApplication.shared
let delegate = TakokitAppDelegate()
app.delegate = delegate
app.run()

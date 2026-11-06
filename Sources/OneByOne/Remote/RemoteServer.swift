import Foundation
import Network

/// Local HTTP remote on 127.0.0.1. Same paths as the Rust build. Commands run on the main thread.
final class RemoteServer {
    static let defaultPort: UInt16 = 8080
    private var listener: NWListener?
    private let handle: (String) -> Void
    let port: UInt16

    /// `handle` receives the command path, for example "/nxt".
    init(port: UInt16 = RemoteServer.defaultPort, handle: @escaping (String) -> Void) {
        self.port = port
        self.handle = handle
    }

    func start() throws {
        let params = NWParameters.tcp
        params.requiredLocalEndpoint = NWEndpoint.hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: port)!)
        let l = try NWListener(using: params)
        l.newConnectionHandler = { [weak self] conn in self?.serve(conn) }
        l.start(queue: .global(qos: .utility))
        listener = l
    }

    func stop() {
        listener?.cancel()
        listener = nil
    }

    private static let commands: Set<String> = ["/res", "/prv", "/nxt", "/rew", "/fwd", "/vdn", "/vup", "/dprv", "/dnxt", "/home"]

    private func serve(_ conn: NWConnection) {
        conn.start(queue: .global(qos: .utility))
        conn.receive(minimumIncompleteLength: 1, maximumLength: 2048) { [weak self] data, _, _, _ in
            guard let self else { conn.cancel(); return }
            let text = String(decoding: data ?? Data(), as: UTF8.self)
            let requestLine = text.components(separatedBy: "\r\n").first ?? ""
            let parts = requestLine.split(separator: " ")
            let path = parts.count > 1 ? String(parts[1]) : "/"
            let response: Data
            if path == "/1by1.m3u" {
                response = Self.http("audio/x-mpegurl",
                                     "#EXTM3U\n#EXTINF:-1,1by1 stream\nhttp://127.0.0.1/1by1.mp3\n")
            } else {
                if Self.commands.contains(path) {
                    DispatchQueue.main.async { self.handle(path) }
                }
                response = Self.http("text/html; charset=utf-8", Self.page())
            }
            conn.send(content: response, completion: .contentProcessed { _ in conn.cancel() })
        }
    }

    private static func http(_ type: String, _ body: String) -> Data {
        let head = "HTTP/1.1 200 OK\r\nContent-Type: \(type)\r\nContent-Length: \(body.utf8.count)\r\nConnection: close\r\n\r\n"
        return Data((head + body).utf8)
    }

    static func page() -> String {
        let links: [(String, String)] = [
            ("/res", "Stop / Resume"), ("/prv", "Previous track"), ("/nxt", "Next track"),
            ("/rew", "Rewind"), ("/fwd", "Forward"), ("/vdn", "Volume down"), ("/vup", "Volume up"),
            ("/dprv", "Previous folder"), ("/dnxt", "Next folder"), ("/home", "Go to current track"),
        ]
        let items = links.map { "<a href=\"\($0.0)\">\($0.1)</a>" }.joined()
        return """
        <!DOCTYPE html><html><head><meta charset="utf-8"><title>1by1</title>
        <style>body{font-family:system-ui,sans-serif;background:#111;color:#eee;margin:0;padding:24px}a{display:block;padding:14px 18px;margin:8px 0;background:#222;color:#eee;text-decoration:none;border-radius:6px}a:hover{background:#333}h1{font-size:18px;font-weight:600}</style></head>
        <body><h1>1by1 remote</h1>\(items)</body></html>
        """
    }
}

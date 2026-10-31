@preconcurrency import Foundation
#if os(iOS)
import Darwin
#endif

@MainActor
class DiscoveryService: NSObject, ObservableObject, @preconcurrency NetServiceBrowserDelegate, @preconcurrency NetServiceDelegate {
    @Published var discoveredDevices: [Device] = []

    private var browser: NetServiceBrowser?
    private var resolvingServices: Set<String> = []

    func startBrowsing() {
        guard browser == nil else { return }
        browser = NetServiceBrowser()
        browser?.delegate = self
        browser?.searchForServices(ofType: "_handoff._tcp.", inDomain: "local.")
    }

    func stopBrowsing() {
        browser?.stop()
        browser = nil
        resolvingServices.removeAll()
        discoveredDevices.removeAll()
    }

    // MARK: - Address parsing

    private func parseAddress(_ data: Data) -> String? {
        let addrData = data.withUnsafeBytes { $0.bindMemory(to: sockaddr.self) }
        guard let firstByte = addrData.first else { return nil }

        if firstByte.sa_family == UInt8(AF_INET) {
            // IPv4
            let sinAddr = data.withUnsafeBytes { ptr -> in_addr? in
                guard ptr.count >= MemoryLayout<sockaddr_in>.size else { return nil }
                let sockAddr = ptr.baseAddress!.assumingMemoryBound(to: sockaddr_in.self).pointee
                return sockAddr.sin_addr
            }
            if var sinAddr = sinAddr {
                var buffer = [CChar](repeating: 0, count: Int(INET_ADDRSTRLEN))
                inet_ntop(AF_INET, &sinAddr, &buffer, socklen_t(INET_ADDRSTRLEN))
                return String(cString: buffer)
            }
        } else if firstByte.sa_family == UInt8(AF_INET6) {
            // IPv6
            let sin6Addr = data.withUnsafeBytes { ptr -> in6_addr? in
                guard ptr.count >= MemoryLayout<sockaddr_in6>.size else { return nil }
                let sockAddr = ptr.baseAddress!.assumingMemoryBound(to: sockaddr_in6.self).pointee
                return sockAddr.sin6_addr
            }
            if var sin6Addr = sin6Addr {
                var buffer = [CChar](repeating: 0, count: Int(INET6_ADDRSTRLEN))
                inet_ntop(AF_INET6, &sin6Addr, &buffer, socklen_t(INET6_ADDRSTRLEN))
                return String(cString: buffer)
            }
        }

        return nil
    }

    // MARK: - NetServiceBrowserDelegate

    func netServiceBrowser(_ browser: NetServiceBrowser, didFind service: NetService, moreComing: Bool) {
        service.delegate = self
        resolvingServices.insert(service.name)
        service.resolve(withTimeout: 10.0)
    }

    func netServiceBrowser(_ browser: NetServiceBrowser, didRemove service: NetService, moreComing: Bool) {
        discoveredDevices.removeAll { $0.name == service.name }
        resolvingServices.remove(service.name)
    }

    // MARK: - NetServiceDelegate

    func netServiceDidResolve(_ sender: NetService) {
        guard let addresses = sender.addresses else {
            resolvingServices.remove(sender.name)
            return
        }

        var ipAddr: String?
        for address in addresses {
            if let addr = parseAddress(address) {
                ipAddr = addr
                break
            }
        }

        guard let addr = ipAddr else {
            resolvingServices.remove(sender.name)
            return
        }

        let device = Device(
            id: "mdns-\(addr)",
            name: sender.name,
            addr: addr,
            port: UInt16(sender.port),
            publicKey: nil
        )

        if !discoveredDevices.contains(where: { $0.addr == addr && $0.port == UInt16(sender.port) }) {
            discoveredDevices.append(device)
        }

        resolvingServices.remove(sender.name)
    }

    func netService(_ sender: NetService, didNotResolve errorDict: [String : NSNumber]) {
        resolvingServices.remove(sender.name)
    }
}

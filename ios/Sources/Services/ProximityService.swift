import Foundation
import CoreBluetooth

class ProximityService: ObservableObject {
    @Published var rssi: Int? = nil
    @Published var isInRange = false

    // ponytail: stub, wire CBCentralManager RSSI readings
    // when daemon implements bluer-based BLE scanning

    private var central: CBCentralManager?

    func startScanning() {
        central = CBCentralManager(delegate: nil, queue: nil)
        // TODO: scan for peripheral with matching UUID, read RSSI
    }

    func stopScanning() {
        central?.stopScan()
        central = nil
    }
}

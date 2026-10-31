// DaemonClient - WebSocket client connecting to handoffd IPC socket
// ponytail: simplified client, assumes daemon exposes WS on unix socket or localhost port

.pragma library

var socket = null;
var reconnectTimer = null;
var devices = [];
var connectedPeers = [];
var clipboardItems = [];

function connect() {
    socket = new WebSocket("ws://127.0.0.1:49200");

    socket.onopen = function() {
        console.log("Connected to handoffd");
        socket.send(JSON.stringify({ type: "subscribe", payload: { channels: ["devices", "clipboard", "notifications"] } }));
    };

    socket.onmessage = function(event) {
        var msg = JSON.parse(event.data);
        handleMessage(msg);
    };

    socket.onclose = function() {
        console.log("Disconnected from handoffd");
        if (!reconnectTimer) {
            reconnectTimer = setTimeout(function() {
                reconnectTimer = null;
                connect();
            }, 3000);
        }
    };
}

function handleMessage(msg) {
    switch (msg.type) {
        case "devices.update":
            devices = msg.payload.devices;
            break;
        case "devices.connected":
            connectedPeers = msg.payload.peers;
            break;
        case "clipboard.history":
            clipboardItems = msg.payload.items;
            break;
        case "notification":
            notificationPopup.show(msg.payload.title, msg.payload.body, msg.payload.actions);
            break;
    }
}

function sendAction(notificationId, actionId) {
    socket.send(JSON.stringify({ type: "notification.action", payload: { notificationId: notificationId, actionId: actionId } }));
}

function copyToClipboard(data) {
    socket.send(JSON.stringify({ type: "clipboard.set", payload: { data: data } }));
}

function sendClipboard(deviceId) {
    socket.send(JSON.stringify({ type: "clipboard.send", payload: { deviceId: deviceId } }));
}

function requestUnlock(deviceId) {
    socket.send(JSON.stringify({ type: "proximity.unlock", payload: { deviceId: deviceId } }));
}

function startDiscovery() {
    socket.send(JSON.stringify({ type: "discovery.start" }));
}

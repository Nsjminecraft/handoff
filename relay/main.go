package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"net/http"
	"sync"
	"time"

	"github.com/gorilla/websocket"
)

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool { return true }, // TODO: restrict in production
}

type Message struct {
	Type      string          `json:"type"`
	ID        string          `json:"id"`
	Timestamp int64           `json:"timestamp"`
	Payload   json.RawMessage `json:"payload"`
}

type Client struct {
	conn       *websocket.Conn
	deviceID   string
	send       chan []byte
	lastActive time.Time
}

type Hub struct {
	clients    map[string]*Client
	register   chan *Client
	unregister chan *Client
	broadcast  chan []byte
	mu         sync.RWMutex
}

func newHub() *Hub {
	return &Hub{
		clients:    make(map[string]*Client),
		register:   make(chan *Client),
		unregister: make(chan *Client),
		broadcast:  make(chan []byte),
	}
}

func (h *Hub) run() {
	for {
		select {
		case c := <-h.register:
			h.mu.Lock()
			h.clients[c.deviceID] = c
			h.mu.Unlock()
			log.Printf("Device registered: %s", c.deviceID)

		case c := <-h.unregister:
			h.mu.Lock()
			if _, ok := h.clients[c.deviceID]; ok {
				delete(h.clients, c.deviceID)
				close(c.send)
			}
			h.mu.Unlock()
			log.Printf("Device unregistered: %s", c.deviceID)

		case msg := <-h.broadcast:
			h.mu.RLock()
			for _, c := range h.clients {
				select {
				case c.send <- msg:
				default:
					close(c.send)
					delete(h.clients, c.deviceID)
				}
			}
			h.mu.RUnlock()
		}
	}
}

func (h *Hub) sendToDevice(deviceID string, data []byte) bool {
	h.mu.RLock()
	defer h.mu.RUnlock()
	if c, ok := h.clients[deviceID]; ok {
		select {
		case c.send <- data:
			return true
		default:
		}
	}
	return false
}

func (h *Hub) handleWS(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("Upgrade error: %v", err)
		return
	}

	// First message must be auth/register with device ID
	_, msg, err := conn.ReadMessage()
	if err != nil {
		conn.Close()
		return
	}

	var auth struct {
		Type     string `json:"type"`
		DeviceID string `json:"device_id"`
		Token    string `json:"token"`
	}
	if err := json.Unmarshal(msg, &auth); err != nil || auth.Type != "auth" || auth.DeviceID == "" {
		conn.WriteMessage(websocket.CloseMessage, websocket.FormatCloseMessage(4001, "auth required"))
		conn.Close()
		return
	}

	// TODO: verify token against stored device keys

	client := &Client{
		conn:       conn,
		deviceID:   auth.DeviceID,
		send:       make(chan []byte, 256),
		lastActive: time.Now(),
	}

	h.register <- client

	go client.writePump()
	client.readPump(h)
}

func (c *Client) readPump(h *Hub) {
	defer func() {
		h.unregister <- c
		c.conn.Close()
	}()

	for {
		_, message, err := c.conn.ReadMessage()
		if err != nil {
			break
		}

		var msg Message
		if err := json.Unmarshal(message, &msg); err != nil {
			continue
		}

		// Route to specific device if payload has target
		var target struct {
			TargetDeviceID string `json:"target_device_id"`
		}
		if json.Unmarshal(msg.Payload, &target) == nil && target.TargetDeviceID != "" {
			if h.sendToDevice(target.TargetDeviceID, message) {
				continue
			}
		}

		// Otherwise broadcast to all peers
		h.broadcast <- message
	}
}

func (c *Client) writePump() {
	for msg := range c.send {
		if err := c.conn.WriteMessage(websocket.BinaryMessage, msg); err != nil {
			c.conn.Close()
			return
		}
	}
}

func main() {
	port := flag.Int("port", 8080, "Relay server port")
	flag.Parse()

	hub := newHub()
	go hub.run()

	http.HandleFunc("/ws", hub.handleWS)

	log.Printf("handoff-relay listening on :%d", *port)
	log.Fatal(http.ListenAndServe(fmt.Sprintf(":%d", *port), nil))
}

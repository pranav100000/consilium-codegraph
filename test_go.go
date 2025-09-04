package main

import (
    "fmt"
    "net/http"
    "log"
)

// Config holds application configuration
type Config struct {
    Port    int
    Host    string
    Debug   bool
}

// Server represents our HTTP server
type Server struct {
    config *Config
    router *http.ServeMux
}

// NewServer creates a new server instance
func NewServer(cfg *Config) *Server {
    return &Server{
        config: cfg,
        router: http.NewServeMux(),
    }
}

// Start starts the HTTP server
func (s *Server) Start() error {
    addr := fmt.Sprintf("%s:%d", s.config.Host, s.config.Port)
    log.Printf("Starting server on %s", addr)
    return http.ListenAndServe(addr, s.router)
}

// HandleHome handles the home route
func (s *Server) HandleHome(w http.ResponseWriter, r *http.Request) {
    fmt.Fprintf(w, "Welcome to the home page!")
}

// HandleAPI handles API requests
func HandleAPI(w http.ResponseWriter, r *http.Request) {
    w.Header().Set("Content-Type", "application/json")
    fmt.Fprintf(w, `{"status": "ok"}`)
}

// User interface for user operations
type User interface {
    GetName() string
    GetEmail() string
    IsActive() bool
}

// UserImpl implements the User interface
type UserImpl struct {
    Name   string
    Email  string
    Active bool
}

// GetName returns the user's name
func (u *UserImpl) GetName() string {
    return u.Name
}

// GetEmail returns the user's email
func (u *UserImpl) GetEmail() string {
    return u.Email
}

// IsActive checks if user is active
func (u UserImpl) IsActive() bool {
    return u.Active
}

const (
    MaxRetries = 3
    Timeout    = 30
)

var (
    DefaultConfig = &Config{
        Port:  8080,
        Host:  "localhost",
        Debug: false,
    }
)

func main() {
    server := NewServer(DefaultConfig)
    server.router.HandleFunc("/", server.HandleHome)
    server.router.HandleFunc("/api", HandleAPI)
    
    if err := server.Start(); err != nil {
        log.Fatal(err)
    }
}
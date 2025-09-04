package main

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"sync"
	"time"
)

// Constants
const (
	MaxRetries    = 3
	DefaultTimeout = 30 * time.Second
	BufferSize    = 100
)

// Custom errors
var (
	ErrNotFound   = errors.New("not found")
	ErrValidation = errors.New("validation failed")
	ErrTimeout    = errors.New("operation timed out")
)

// Interfaces
type Cacheable interface {
	GetCacheKey() string
	Serialize() ([]byte, error)
}

type Repository interface {
	Find(id string) (interface{}, error)
	Save(item interface{}) error
	Delete(id string) error
}

// User struct with tags
type User struct {
	ID        string    `json:"id" db:"id"`
	Name      string    `json:"name" db:"name"`
	Email     string    `json:"email" db:"email"`
	Role      UserRole  `json:"role" db:"role"`
	IsActive  bool      `json:"is_active" db:"is_active"`
	CreatedAt time.Time `json:"created_at" db:"created_at"`
	Metadata  map[string]interface{} `json:"metadata,omitempty"`
}

// User methods
func (u *User) GetCacheKey() string {
	return fmt.Sprintf("user:%s", u.ID)
}

func (u *User) Serialize() ([]byte, error) {
	return json.Marshal(u)
}

func (u *User) Validate() error {
	if u.Name == "" {
		return fmt.Errorf("%w: name is required", ErrValidation)
	}
	if u.Email == "" {
		return fmt.Errorf("%w: email is required", ErrValidation)
	}
	return nil
}

func (u *User) IsAdmin() bool {
	return u.Role == RoleAdmin
}

// Enum using iota
type UserRole int

const (
	RoleGuest UserRole = iota
	RoleUser
	RoleAdmin
)

func (r UserRole) String() string {
	return [...]string{"Guest", "User", "Admin"}[r]
}

// Generic type using type parameters (Go 1.18+)
type Cache[T any] struct {
	mu    sync.RWMutex
	items map[string]T
	ttl   time.Duration
}

func NewCache[T any](ttl time.Duration) *Cache[T] {
	return &Cache[T]{
		items: make(map[string]T),
		ttl:   ttl,
	}
}

func (c *Cache[T]) Set(key string, value T) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.items[key] = value
}

func (c *Cache[T]) Get(key string) (T, bool) {
	c.mu.RLock()
	defer c.mu.RUnlock()
	value, ok := c.items[key]
	return value, ok
}

// Service with embedded struct
type BaseService struct {
	logger *log.Logger
	cache  *Cache[string]
}

func (s *BaseService) Log(msg string) {
	s.logger.Printf("[%s] %s", time.Now().Format(time.RFC3339), msg)
}

type UserService struct {
	BaseService
	repo  UserRepository
	mutex sync.RWMutex
	users map[string]*User
}

type UserRepository interface {
	Repository
	FindByEmail(email string) (*User, error)
	FindActiveUsers() ([]*User, error)
}

// Constructor function
func NewUserService(repo UserRepository, logger *log.Logger) *UserService {
	return &UserService{
		BaseService: BaseService{
			logger: logger,
			cache:  NewCache[string](5 * time.Minute),
		},
		repo:  repo,
		users: make(map[string]*User),
	}
}

// Methods with various signatures
func (s *UserService) CreateUser(ctx context.Context, name, email string) (*User, error) {
	user := &User{
		ID:        generateID(),
		Name:      name,
		Email:     email,
		Role:      RoleUser,
		IsActive:  true,
		CreatedAt: time.Now(),
	}
	
	if err := user.Validate(); err != nil {
		return nil, err
	}
	
	s.mutex.Lock()
	s.users[user.ID] = user
	s.mutex.Unlock()
	
	s.Log(fmt.Sprintf("Created user: %s", user.ID))
	
	if err := s.repo.Save(user); err != nil {
		return nil, fmt.Errorf("failed to save user: %w", err)
	}
	
	return user, nil
}

func (s *UserService) GetUser(id string) (*User, error) {
	// Check cache first
	if cached, ok := s.cache.Get(id); ok {
		var user User
		if err := json.Unmarshal([]byte(cached), &user); err == nil {
			return &user, nil
		}
	}
	
	s.mutex.RLock()
	user, ok := s.users[id]
	s.mutex.RUnlock()
	
	if !ok {
		return nil, ErrNotFound
	}
	
	// Cache the result
	if data, err := user.Serialize(); err == nil {
		s.cache.Set(id, string(data))
	}
	
	return user, nil
}

// Variadic function
func (s *UserService) GetUsers(ids ...string) ([]*User, error) {
	users := make([]*User, 0, len(ids))
	
	for _, id := range ids {
		user, err := s.GetUser(id)
		if err != nil {
			if errors.Is(err, ErrNotFound) {
				continue
			}
			return nil, err
		}
		users = append(users, user)
	}
	
	return users, nil
}

// Method with multiple return values
func (s *UserService) FindUser(email string) (*User, bool, error) {
	user, err := s.repo.FindByEmail(email)
	if err != nil {
		if errors.Is(err, ErrNotFound) {
			return nil, false, nil
		}
		return nil, false, err
	}
	return user, true, nil
}

// Goroutines and channels
func (s *UserService) ProcessUsersAsync(ctx context.Context) <-chan *User {
	ch := make(chan *User, BufferSize)
	
	go func() {
		defer close(ch)
		
		s.mutex.RLock()
		users := make([]*User, 0, len(s.users))
		for _, user := range s.users {
			users = append(users, user)
		}
		s.mutex.RUnlock()
		
		for _, user := range users {
			select {
			case <-ctx.Done():
				s.Log("Processing cancelled")
				return
			case ch <- user:
				// Process user
			}
		}
	}()
	
	return ch
}

// Defer, panic, and recover
func (s *UserService) BulkOperation(fn func(*User) error) (err error) {
	defer func() {
		if r := recover(); r != nil {
			err = fmt.Errorf("panic recovered: %v", r)
			s.Log(fmt.Sprintf("Recovered from panic: %v", r))
		}
	}()
	
	s.mutex.RLock()
	defer s.mutex.RUnlock()
	
	for _, user := range s.users {
		if err := fn(user); err != nil {
			return fmt.Errorf("operation failed for user %s: %w", user.ID, err)
		}
	}
	
	return nil
}

// Function type and closure
type UserPredicate func(*User) bool

func (s *UserService) FilterUsers(predicate UserPredicate) []*User {
	var filtered []*User
	
	s.mutex.RLock()
	defer s.mutex.RUnlock()
	
	for _, user := range s.users {
		if predicate(user) {
			filtered = append(filtered, user)
		}
	}
	
	return filtered
}

// Higher-order function
func WithRetry(maxRetries int, fn func() error) error {
	var err error
	for i := 0; i < maxRetries; i++ {
		if err = fn(); err == nil {
			return nil
		}
		time.Sleep(time.Duration(i+1) * time.Second)
	}
	return fmt.Errorf("failed after %d retries: %w", maxRetries, err)
}

// Worker pool pattern
func (s *UserService) ProcessUsersBatch(users []*User, workerCount int) error {
	jobs := make(chan *User, len(users))
	results := make(chan error, len(users))
	
	// Start workers
	var wg sync.WaitGroup
	for i := 0; i < workerCount; i++ {
		wg.Add(1)
		go func(workerID int) {
			defer wg.Done()
			for user := range jobs {
				s.Log(fmt.Sprintf("Worker %d processing user %s", workerID, user.ID))
				// Simulate processing
				time.Sleep(100 * time.Millisecond)
				results <- nil
			}
		}(i)
	}
	
	// Send jobs
	for _, user := range users {
		jobs <- user
	}
	close(jobs)
	
	// Wait for completion
	wg.Wait()
	close(results)
	
	// Check results
	for err := range results {
		if err != nil {
			return err
		}
	}
	
	return nil
}

// Struct embedding and composition
type AuditableUser struct {
	User
	UpdatedAt time.Time
	UpdatedBy string
}

func (au *AuditableUser) Audit(by string) {
	au.UpdatedAt = time.Now()
	au.UpdatedBy = by
}

// Helper functions
func generateID() string {
	return fmt.Sprintf("usr_%d", time.Now().UnixNano())
}

// Init function
func init() {
	log.SetPrefix("[UserService] ")
	log.SetFlags(log.LstdFlags | log.Lshortfile)
}

// Main function demonstrating usage
func main() {
	logger := log.New(log.Writer(), "[MAIN] ", log.LstdFlags)
	
	// Mock repository
	repo := &mockRepository{}
	
	// Create service
	service := NewUserService(repo, logger)
	
	// Create users
	ctx := context.Background()
	user1, err := service.CreateUser(ctx, "Alice", "alice@example.com")
	if err != nil {
		log.Fatal(err)
	}
	
	user2, err := service.CreateUser(ctx, "Bob", "bob@example.com")
	if err != nil {
		log.Fatal(err)
	}
	
	// Get users
	users, err := service.GetUsers(user1.ID, user2.ID)
	if err != nil {
		log.Fatal(err)
	}
	
	fmt.Printf("Found %d users\n", len(users))
	
	// Filter users
	admins := service.FilterUsers(func(u *User) bool {
		return u.IsAdmin()
	})
	
	fmt.Printf("Found %d admins\n", len(admins))
	
	// Process async
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	
	userChan := service.ProcessUsersAsync(ctx)
	for user := range userChan {
		fmt.Printf("Processed user: %s\n", user.Name)
	}
	
	// Bulk operation with retry
	err = WithRetry(MaxRetries, func() error {
		return service.BulkOperation(func(u *User) error {
			fmt.Printf("Bulk processing: %s\n", u.Name)
			return nil
		})
	})
	
	if err != nil {
		log.Printf("Bulk operation failed: %v", err)
	}
	
	// Worker pool
	allUsers := service.FilterUsers(func(u *User) bool { return true })
	if err := service.ProcessUsersBatch(allUsers, 3); err != nil {
		log.Printf("Batch processing failed: %v", err)
	}
	
	fmt.Println("All operations completed")
}

// Mock repository for testing
type mockRepository struct {
	mu    sync.Mutex
	data  map[string]interface{}
}

func (r *mockRepository) Find(id string) (interface{}, error) {
	r.mu.Lock()
	defer r.mu.Unlock()
	
	if item, ok := r.data[id]; ok {
		return item, nil
	}
	return nil, ErrNotFound
}

func (r *mockRepository) Save(item interface{}) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	
	if r.data == nil {
		r.data = make(map[string]interface{})
	}
	
	if user, ok := item.(*User); ok {
		r.data[user.ID] = user
		return nil
	}
	
	return ErrValidation
}

func (r *mockRepository) Delete(id string) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	
	delete(r.data, id)
	return nil
}

func (r *mockRepository) FindByEmail(email string) (*User, error) {
	r.mu.Lock()
	defer r.mu.Unlock()
	
	for _, item := range r.data {
		if user, ok := item.(*User); ok && user.Email == email {
			return user, nil
		}
	}
	
	return nil, ErrNotFound
}

func (r *mockRepository) FindActiveUsers() ([]*User, error) {
	r.mu.Lock()
	defer r.mu.Unlock()
	
	var users []*User
	for _, item := range r.data {
		if user, ok := item.(*User); ok && user.IsActive {
			users = append(users, user)
		}
	}
	
	return users, nil
}
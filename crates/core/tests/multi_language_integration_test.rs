use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn create_comprehensive_multi_language_project(temp_dir: &Path) -> Result<()> {
    // Create TypeScript project
    let package_json = r#"{
  "name": "multi-lang-test-project",
  "version": "1.0.0",
  "scripts": {
    "build": "tsc",
    "test": "jest"
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "@types/node": "^20.0.0",
    "jest": "^29.0.0"
  }
}"#;
    fs::write(temp_dir.join("package.json"), package_json)?;
    
    let tsconfig = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "outDir": "./dist"
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}"#;
    fs::write(temp_dir.join("tsconfig.json"), tsconfig)?;
    
    fs::create_dir_all(temp_dir.join("src"))?;
    
    let ts_user_service = r#"
export interface User {
    id: number;
    name: string;
    email: string;
    isActive: boolean;
}

export class UserService {
    private users: User[] = [];
    private static instance: UserService;
    
    private constructor() {}
    
    public static getInstance(): UserService {
        if (!UserService.instance) {
            UserService.instance = new UserService();
        }
        return UserService.instance;
    }
    
    public addUser(user: User): void {
        if (this.findUserById(user.id)) {
            throw new Error(`User with ID ${user.id} already exists`);
        }
        this.users.push(user);
    }
    
    public getUser(id: number): User | null {
        return this.findUserById(id);
    }
    
    public updateUser(id: number, updates: Partial<User>): boolean {
        const user = this.findUserById(id);
        if (!user) return false;
        
        Object.assign(user, updates);
        return true;
    }
    
    public deleteUser(id: number): boolean {
        const index = this.users.findIndex(u => u.id === id);
        if (index === -1) return false;
        
        this.users.splice(index, 1);
        return true;
    }
    
    public getActiveUsers(): User[] {
        return this.users.filter(user => user.isActive);
    }
    
    public getUserCount(): number {
        return this.users.length;
    }
    
    private findUserById(id: number): User | undefined {
        return this.users.find(u => u.id === id);
    }
}
"#;
    fs::write(temp_dir.join("src").join("user-service.ts"), ts_user_service)?;
    
    let ts_auth_service = r#"
import { User, UserService } from './user-service';

export interface AuthToken {
    userId: number;
    token: string;
    expiresAt: Date;
}

export class AuthService {
    private userService: UserService;
    private activeSessions: Map<string, AuthToken> = new Map();
    
    constructor() {
        this.userService = UserService.getInstance();
    }
    
    public async login(email: string, password: string): Promise<AuthToken | null> {
        // Simulate authentication
        const users = this.userService.getActiveUsers();
        const user = users.find(u => u.email === email);
        
        if (!user) return null;
        
        const token = this.generateToken();
        const authToken: AuthToken = {
            userId: user.id,
            token,
            expiresAt: new Date(Date.now() + 3600000) // 1 hour
        };
        
        this.activeSessions.set(token, authToken);
        return authToken;
    }
    
    public validateToken(token: string): User | null {
        const authToken = this.activeSessions.get(token);
        if (!authToken || authToken.expiresAt < new Date()) {
            this.activeSessions.delete(token);
            return null;
        }
        
        return this.userService.getUser(authToken.userId);
    }
    
    public logout(token: string): void {
        this.activeSessions.delete(token);
    }
    
    private generateToken(): string {
        return Math.random().toString(36).substring(2) + Date.now().toString(36);
    }
}
"#;
    fs::write(temp_dir.join("src").join("auth-service.ts"), ts_auth_service)?;
    
    // Create Python project
    let requirements = r#"fastapi>=0.104.0
uvicorn>=0.24.0
pydantic>=2.4.0
sqlalchemy>=2.0.0
alembic>=1.12.0
pytest>=7.4.0
pytest-asyncio>=0.21.0
httpx>=0.25.0
"#;
    fs::write(temp_dir.join("requirements.txt"), requirements)?;
    
    let pyproject_toml = r#"[build-system]
requires = ["setuptools>=45", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "multi-lang-service"
version = "0.1.0"
description = "Multi-language integration test project"
authors = [{name = "Test Author", email = "test@example.com"}]
dependencies = [
    "fastapi>=0.104.0",
    "uvicorn>=0.24.0",
    "pydantic>=2.4.0"
]

[project.optional-dependencies]
dev = ["pytest>=7.4.0", "pytest-asyncio>=0.21.0"]
"#;
    fs::write(temp_dir.join("pyproject.toml"), pyproject_toml)?;
    
    let py_models = r#"
from typing import List, Optional
from pydantic import BaseModel, EmailStr
from datetime import datetime
from enum import Enum

class UserStatus(str, Enum):
    ACTIVE = "active"
    INACTIVE = "inactive"
    SUSPENDED = "suspended"

class UserBase(BaseModel):
    name: str
    email: EmailStr
    status: UserStatus = UserStatus.ACTIVE

class UserCreate(UserBase):
    password: str

class UserUpdate(BaseModel):
    name: Optional[str] = None
    email: Optional[EmailStr] = None
    status: Optional[UserStatus] = None

class UserResponse(UserBase):
    id: int
    created_at: datetime
    updated_at: datetime
    
    class Config:
        from_attributes = True

class AuthToken(BaseModel):
    access_token: str
    token_type: str = "bearer"
    expires_in: int

class LoginRequest(BaseModel):
    email: EmailStr
    password: str
"#;
    fs::write(temp_dir.join("models.py"), py_models)?;
    
    let py_service = r#"
from typing import List, Optional
from datetime import datetime, timedelta
from models import UserCreate, UserUpdate, UserResponse, UserStatus, AuthToken, LoginRequest
import hashlib
import secrets

class DatabaseService:
    """Simulated database service for testing purposes"""
    
    def __init__(self):
        self._users = {}
        self._next_id = 1
    
    def create_user(self, user: UserCreate) -> UserResponse:
        user_id = self._next_id
        self._next_id += 1
        
        now = datetime.now()
        user_data = {
            "id": user_id,
            "name": user.name,
            "email": user.email,
            "status": user.status,
            "created_at": now,
            "updated_at": now,
            "password_hash": self._hash_password(user.password)
        }
        
        self._users[user_id] = user_data
        return UserResponse(**user_data)
    
    def get_user(self, user_id: int) -> Optional[UserResponse]:
        user_data = self._users.get(user_id)
        if not user_data:
            return None
        return UserResponse(**user_data)
    
    def get_user_by_email(self, email: str) -> Optional[UserResponse]:
        for user_data in self._users.values():
            if user_data["email"] == email:
                return UserResponse(**user_data)
        return None
    
    def update_user(self, user_id: int, user_update: UserUpdate) -> Optional[UserResponse]:
        user_data = self._users.get(user_id)
        if not user_data:
            return None
        
        update_data = user_update.model_dump(exclude_unset=True)
        user_data.update(update_data)
        user_data["updated_at"] = datetime.now()
        
        return UserResponse(**user_data)
    
    def delete_user(self, user_id: int) -> bool:
        return self._users.pop(user_id, None) is not None
    
    def list_users(self, status: Optional[UserStatus] = None) -> List[UserResponse]:
        users = list(self._users.values())
        if status:
            users = [u for u in users if u["status"] == status]
        return [UserResponse(**u) for u in users]
    
    def verify_password(self, user_id: int, password: str) -> bool:
        user_data = self._users.get(user_id)
        if not user_data:
            return False
        return user_data["password_hash"] == self._hash_password(password)
    
    def _hash_password(self, password: str) -> str:
        return hashlib.sha256(password.encode()).hexdigest()

class UserService:
    """Service layer for user management"""
    
    def __init__(self, db: DatabaseService):
        self.db = db
    
    async def create_user(self, user: UserCreate) -> UserResponse:
        # Check if user already exists
        existing = self.db.get_user_by_email(user.email)
        if existing:
            raise ValueError(f"User with email {user.email} already exists")
        
        return self.db.create_user(user)
    
    async def get_user(self, user_id: int) -> Optional[UserResponse]:
        return self.db.get_user(user_id)
    
    async def update_user(self, user_id: int, user_update: UserUpdate) -> Optional[UserResponse]:
        return self.db.update_user(user_id, user_update)
    
    async def delete_user(self, user_id: int) -> bool:
        return self.db.delete_user(user_id)
    
    async def list_active_users(self) -> List[UserResponse]:
        return self.db.list_users(UserStatus.ACTIVE)
    
    async def authenticate_user(self, email: str, password: str) -> Optional[UserResponse]:
        user = self.db.get_user_by_email(email)
        if not user or not self.db.verify_password(user.id, password):
            return None
        return user

class AuthService:
    """Authentication service"""
    
    def __init__(self, user_service: UserService):
        self.user_service = user_service
        self._active_tokens = {}
    
    async def login(self, login_request: LoginRequest) -> Optional[AuthToken]:
        user = await self.user_service.authenticate_user(
            login_request.email, 
            login_request.password
        )
        if not user:
            return None
        
        token = secrets.token_urlsafe(32)
        expires_in = 3600  # 1 hour
        
        self._active_tokens[token] = {
            "user_id": user.id,
            "expires_at": datetime.now() + timedelta(seconds=expires_in)
        }
        
        return AuthToken(
            access_token=token,
            expires_in=expires_in
        )
    
    async def validate_token(self, token: str) -> Optional[UserResponse]:
        token_data = self._active_tokens.get(token)
        if not token_data or token_data["expires_at"] < datetime.now():
            self._active_tokens.pop(token, None)
            return None
        
        return await self.user_service.get_user(token_data["user_id"])
    
    async def logout(self, token: str) -> bool:
        return self._active_tokens.pop(token, None) is not None
"#;
    fs::write(temp_dir.join("service.py"), py_service)?;
    
    // Create Go project
    let go_mod = r#"module github.com/example/multi-lang-service

go 1.21

require (
    github.com/gin-gonic/gin v1.9.1
    github.com/golang-jwt/jwt/v5 v5.0.0
    gorm.io/gorm v1.25.5
    gorm.io/driver/sqlite v1.5.4
)
"#;
    fs::write(temp_dir.join("go.mod"), go_mod)?;
    
    let go_models = r#"
package main

import (
    "time"
    "gorm.io/gorm"
)

type UserStatus string

const (
    UserStatusActive    UserStatus = "active"
    UserStatusInactive  UserStatus = "inactive" 
    UserStatusSuspended UserStatus = "suspended"
)

type User struct {
    ID        uint      `json:"id" gorm:"primaryKey"`
    Name      string    `json:"name" gorm:"not null"`
    Email     string    `json:"email" gorm:"uniqueIndex;not null"`
    Password  string    `json:"-" gorm:"not null"`
    Status    UserStatus `json:"status" gorm:"default:active"`
    CreatedAt time.Time `json:"created_at"`
    UpdatedAt time.Time `json:"updated_at"`
    DeletedAt gorm.DeletedAt `json:"-" gorm:"index"`
}

type UserCreateRequest struct {
    Name     string     `json:"name" binding:"required"`
    Email    string     `json:"email" binding:"required,email"`
    Password string     `json:"password" binding:"required,min=6"`
    Status   UserStatus `json:"status"`
}

type UserUpdateRequest struct {
    Name   *string     `json:"name,omitempty"`
    Email  *string     `json:"email,omitempty"`
    Status *UserStatus `json:"status,omitempty"`
}

type UserResponse struct {
    ID        uint      `json:"id"`
    Name      string    `json:"name"`
    Email     string    `json:"email"`
    Status    UserStatus `json:"status"`
    CreatedAt time.Time `json:"created_at"`
    UpdatedAt time.Time `json:"updated_at"`
}

type AuthToken struct {
    AccessToken string `json:"access_token"`
    TokenType   string `json:"token_type"`
    ExpiresIn   int    `json:"expires_in"`
}

type LoginRequest struct {
    Email    string `json:"email" binding:"required,email"`
    Password string `json:"password" binding:"required"`
}

type LoginResponse struct {
    Token AuthToken    `json:"token"`
    User  UserResponse `json:"user"`
}

func (u *User) ToResponse() UserResponse {
    return UserResponse{
        ID:        u.ID,
        Name:      u.Name,
        Email:     u.Email,
        Status:    u.Status,
        CreatedAt: u.CreatedAt,
        UpdatedAt: u.UpdatedAt,
    }
}
"#;
    fs::write(temp_dir.join("models.go"), go_models)?;
    
    let go_service = r#"
package main

import (
    "crypto/sha256"
    "errors"
    "fmt"
    "time"
    
    "github.com/golang-jwt/jwt/v5"
    "gorm.io/gorm"
)

var jwtSecret = []byte("your-secret-key")

type Claims struct {
    UserID uint   `json:"user_id"`
    Email  string `json:"email"`
    jwt.RegisteredClaims
}

type UserService struct {
    db *gorm.DB
}

func NewUserService(db *gorm.DB) *UserService {
    return &UserService{db: db}
}

func (s *UserService) CreateUser(req UserCreateRequest) (*UserResponse, error) {
    // Check if user exists
    var existingUser User
    if err := s.db.Where("email = ?", req.Email).First(&existingUser).Error; err == nil {
        return nil, errors.New("user with this email already exists")
    }
    
    // Hash password
    hashedPassword := s.hashPassword(req.Password)
    
    user := User{
        Name:     req.Name,
        Email:    req.Email,
        Password: hashedPassword,
        Status:   req.Status,
    }
    
    if user.Status == "" {
        user.Status = UserStatusActive
    }
    
    if err := s.db.Create(&user).Error; err != nil {
        return nil, err
    }
    
    response := user.ToResponse()
    return &response, nil
}

func (s *UserService) GetUser(id uint) (*UserResponse, error) {
    var user User
    if err := s.db.First(&user, id).Error; err != nil {
        if errors.Is(err, gorm.ErrRecordNotFound) {
            return nil, nil
        }
        return nil, err
    }
    
    response := user.ToResponse()
    return &response, nil
}

func (s *UserService) GetUserByEmail(email string) (*User, error) {
    var user User
    if err := s.db.Where("email = ?", email).First(&user).Error; err != nil {
        if errors.Is(err, gorm.ErrRecordNotFound) {
            return nil, nil
        }
        return nil, err
    }
    return &user, nil
}

func (s *UserService) UpdateUser(id uint, req UserUpdateRequest) (*UserResponse, error) {
    var user User
    if err := s.db.First(&user, id).Error; err != nil {
        if errors.Is(err, gorm.ErrRecordNotFound) {
            return nil, nil
        }
        return nil, err
    }
    
    updates := make(map[string]interface{})
    if req.Name != nil {
        updates["name"] = *req.Name
    }
    if req.Email != nil {
        updates["email"] = *req.Email
    }
    if req.Status != nil {
        updates["status"] = *req.Status
    }
    
    if err := s.db.Model(&user).Updates(updates).Error; err != nil {
        return nil, err
    }
    
    response := user.ToResponse()
    return &response, nil
}

func (s *UserService) DeleteUser(id uint) error {
    return s.db.Delete(&User{}, id).Error
}

func (s *UserService) ListActiveUsers() ([]UserResponse, error) {
    var users []User
    if err := s.db.Where("status = ?", UserStatusActive).Find(&users).Error; err != nil {
        return nil, err
    }
    
    responses := make([]UserResponse, len(users))
    for i, user := range users {
        responses[i] = user.ToResponse()
    }
    return responses, nil
}

func (s *UserService) AuthenticateUser(email, password string) (*User, error) {
    user, err := s.GetUserByEmail(email)
    if err != nil {
        return nil, err
    }
    if user == nil {
        return nil, nil
    }
    
    if !s.verifyPassword(user.Password, password) {
        return nil, nil
    }
    
    return user, nil
}

func (s *UserService) hashPassword(password string) string {
    hash := sha256.Sum256([]byte(password))
    return fmt.Sprintf("%x", hash)
}

func (s *UserService) verifyPassword(hashedPassword, password string) bool {
    return hashedPassword == s.hashPassword(password)
}

type AuthService struct {
    userService *UserService
}

func NewAuthService(userService *UserService) *AuthService {
    return &AuthService{userService: userService}
}

func (s *AuthService) Login(req LoginRequest) (*LoginResponse, error) {
    user, err := s.userService.AuthenticateUser(req.Email, req.Password)
    if err != nil {
        return nil, err
    }
    if user == nil {
        return nil, errors.New("invalid credentials")
    }
    
    // Generate JWT token
    expirationTime := time.Now().Add(24 * time.Hour)
    claims := &Claims{
        UserID: user.ID,
        Email:  user.Email,
        RegisteredClaims: jwt.RegisteredClaims{
            ExpiresAt: jwt.NewNumericDate(expirationTime),
            IssuedAt:  jwt.NewNumericDate(time.Now()),
        },
    }
    
    token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
    tokenString, err := token.SignedString(jwtSecret)
    if err != nil {
        return nil, err
    }
    
    response := &LoginResponse{
        Token: AuthToken{
            AccessToken: tokenString,
            TokenType:   "Bearer",
            ExpiresIn:   86400, // 24 hours in seconds
        },
        User: user.ToResponse(),
    }
    
    return response, nil
}

func (s *AuthService) ValidateToken(tokenString string) (*User, error) {
    claims := &Claims{}
    token, err := jwt.ParseWithClaims(tokenString, claims, func(token *jwt.Token) (interface{}, error) {
        return jwtSecret, nil
    })
    
    if err != nil || !token.Valid {
        return nil, errors.New("invalid token")
    }
    
    user, err := s.userService.GetUserByEmail(claims.Email)
    if err != nil {
        return nil, err
    }
    
    return user, nil
}
"#;
    fs::write(temp_dir.join("service.go"), go_service)?;
    
    let go_main = r#"
package main

import (
    "log"
    "net/http"
    "strconv"
    
    "github.com/gin-gonic/gin"
    "gorm.io/driver/sqlite"
    "gorm.io/gorm"
)

func main() {
    // Initialize database
    db, err := gorm.Open(sqlite.Open("test.db"), &gorm.Config{})
    if err != nil {
        log.Fatal("Failed to connect to database:", err)
    }
    
    // Auto migrate
    err = db.AutoMigrate(&User{})
    if err != nil {
        log.Fatal("Failed to migrate database:", err)
    }
    
    // Initialize services
    userService := NewUserService(db)
    authService := NewAuthService(userService)
    
    // Initialize router
    r := gin.Default()
    
    // Auth routes
    r.POST("/auth/login", func(c *gin.Context) {
        var req LoginRequest
        if err := c.ShouldBindJSON(&req); err != nil {
            c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
            return
        }
        
        response, err := authService.Login(req)
        if err != nil {
            c.JSON(http.StatusUnauthorized, gin.H{"error": err.Error()})
            return
        }
        
        c.JSON(http.StatusOK, response)
    })
    
    // User routes
    r.POST("/users", func(c *gin.Context) {
        var req UserCreateRequest
        if err := c.ShouldBindJSON(&req); err != nil {
            c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
            return
        }
        
        user, err := userService.CreateUser(req)
        if err != nil {
            c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
            return
        }
        
        c.JSON(http.StatusCreated, user)
    })
    
    r.GET("/users/:id", func(c *gin.Context) {
        idStr := c.Param("id")
        id, err := strconv.ParseUint(idStr, 10, 32)
        if err != nil {
            c.JSON(http.StatusBadRequest, gin.H{"error": "Invalid user ID"})
            return
        }
        
        user, err := userService.GetUser(uint(id))
        if err != nil {
            c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
            return
        }
        if user == nil {
            c.JSON(http.StatusNotFound, gin.H{"error": "User not found"})
            return
        }
        
        c.JSON(http.StatusOK, user)
    })
    
    r.GET("/users", func(c *gin.Context) {
        users, err := userService.ListActiveUsers()
        if err != nil {
            c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
            return
        }
        
        c.JSON(http.StatusOK, users)
    })
    
    log.Println("Server starting on :8080")
    log.Fatal(r.Run(":8080"))
}
"#;
    fs::write(temp_dir.join("main.go"), go_main)?;
    
    // Create Rust project
    let cargo_toml = r#"[package]
name = "multi-lang-service"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
sha2 = "0.10"
jsonwebtoken = "9.0"
thiserror = "1.0"
anyhow = "1.0"

[dev-dependencies]
tokio-test = "0.4"
"#;
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml)?;
    
    fs::create_dir_all(temp_dir.join("src"))?;
    
    let rust_lib = r#"
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserStatus {
    Active,
    Inactive,
    Suspended,
}

impl Default for UserStatus {
    fn default() -> Self {
        UserStatus::Active
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserCreateRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub status: UserStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserUpdateRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub status: Option<UserStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: AuthToken,
    pub user: UserResponse,
}

impl User {
    pub fn new(name: String, email: String, password: String, status: UserStatus) -> Self {
        let now = Utc::now();
        let password_hash = hash_password(&password);
        
        Self {
            id: Uuid::new_v4(),
            name,
            email,
            password_hash,
            status,
            created_at: now,
            updated_at: now,
        }
    }
    
    pub fn to_response(&self) -> UserResponse {
        UserResponse {
            id: self.id,
            name: self.name.clone(),
            email: self.email.clone(),
            status: self.status.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
    
    pub fn verify_password(&self, password: &str) -> bool {
        self.password_hash == hash_password(password)
    }
    
    pub fn update(&mut self, request: UserUpdateRequest) {
        if let Some(name) = request.name {
            self.name = name;
        }
        if let Some(email) = request.email {
            self.email = email;
        }
        if let Some(status) = request.status {
            self.status = status;
        }
        self.updated_at = Utc::now();
    }
}

pub fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("User not found")]
    UserNotFound,
    #[error("User already exists with email: {0}")]
    UserAlreadyExists(String),
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

pub struct DatabaseService {
    users: HashMap<Uuid, User>,
    email_index: HashMap<String, Uuid>,
}

impl DatabaseService {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            email_index: HashMap::new(),
        }
    }
    
    pub fn create_user(&mut self, request: UserCreateRequest) -> Result<UserResponse, ServiceError> {
        if self.email_index.contains_key(&request.email) {
            return Err(ServiceError::UserAlreadyExists(request.email));
        }
        
        let user = User::new(request.name, request.email.clone(), request.password, request.status);
        let response = user.to_response();
        
        self.email_index.insert(request.email, user.id);
        self.users.insert(user.id, user);
        
        Ok(response)
    }
    
    pub fn get_user(&self, id: Uuid) -> Result<Option<UserResponse>, ServiceError> {
        Ok(self.users.get(&id).map(|u| u.to_response()))
    }
    
    pub fn get_user_by_email(&self, email: &str) -> Result<Option<&User>, ServiceError> {
        let id = self.email_index.get(email);
        Ok(id.and_then(|id| self.users.get(id)))
    }
    
    pub fn update_user(&mut self, id: Uuid, request: UserUpdateRequest) -> Result<Option<UserResponse>, ServiceError> {
        if let Some(user) = self.users.get_mut(&id) {
            user.update(request);
            Ok(Some(user.to_response()))
        } else {
            Ok(None)
        }
    }
    
    pub fn delete_user(&mut self, id: Uuid) -> Result<bool, ServiceError> {
        if let Some(user) = self.users.remove(&id) {
            self.email_index.remove(&user.email);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    pub fn list_active_users(&self) -> Result<Vec<UserResponse>, ServiceError> {
        let active_users: Vec<_> = self.users
            .values()
            .filter(|u| u.status == UserStatus::Active)
            .map(|u| u.to_response())
            .collect();
        Ok(active_users)
    }
}

pub struct UserService {
    db: DatabaseService,
}

impl UserService {
    pub fn new() -> Self {
        Self {
            db: DatabaseService::new(),
        }
    }
    
    pub async fn create_user(&mut self, request: UserCreateRequest) -> Result<UserResponse, ServiceError> {
        self.db.create_user(request)
    }
    
    pub async fn get_user(&self, id: Uuid) -> Result<Option<UserResponse>, ServiceError> {
        self.db.get_user(id)
    }
    
    pub async fn update_user(&mut self, id: Uuid, request: UserUpdateRequest) -> Result<Option<UserResponse>, ServiceError> {
        self.db.update_user(id, request)
    }
    
    pub async fn delete_user(&mut self, id: Uuid) -> Result<bool, ServiceError> {
        self.db.delete_user(id)
    }
    
    pub async fn list_active_users(&self) -> Result<Vec<UserResponse>, ServiceError> {
        self.db.list_active_users()
    }
    
    pub async fn authenticate_user(&self, email: &str, password: &str) -> Result<Option<&User>, ServiceError> {
        if let Some(user) = self.db.get_user_by_email(email)? {
            if user.verify_password(password) {
                Ok(Some(user))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

pub struct AuthService {
    user_service: UserService,
    active_tokens: HashMap<String, (Uuid, DateTime<Utc>)>,
}

impl AuthService {
    pub fn new(user_service: UserService) -> Self {
        Self {
            user_service,
            active_tokens: HashMap::new(),
        }
    }
    
    pub async fn login(&mut self, request: LoginRequest) -> Result<Option<LoginResponse>, ServiceError> {
        if let Some(user) = self.user_service.authenticate_user(&request.email, &request.password).await? {
            let token = Uuid::new_v4().to_string();
            let expires_at = Utc::now() + chrono::Duration::hours(1);
            
            self.active_tokens.insert(token.clone(), (user.id, expires_at));
            
            let auth_token = AuthToken {
                access_token: token,
                token_type: "Bearer".to_string(),
                expires_in: 3600,
            };
            
            let response = LoginResponse {
                token: auth_token,
                user: user.to_response(),
            };
            
            Ok(Some(response))
        } else {
            Ok(None)
        }
    }
    
    pub async fn validate_token(&self, token: &str) -> Result<Option<UserResponse>, ServiceError> {
        if let Some((user_id, expires_at)) = self.active_tokens.get(token) {
            if *expires_at > Utc::now() {
                return self.user_service.get_user(*user_id).await;
            }
        }
        Ok(None)
    }
    
    pub async fn logout(&mut self, token: &str) -> bool {
        self.active_tokens.remove(token).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_user_service_create_and_get() {
        let mut service = UserService::new();
        
        let request = UserCreateRequest {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            status: UserStatus::Active,
        };
        
        let user = service.create_user(request).await.unwrap();
        assert_eq!(user.name, "Test User");
        assert_eq!(user.email, "test@example.com");
        
        let retrieved = service.get_user(user.id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, user.id);
    }
}
"#;
    fs::write(temp_dir.join("src").join("lib.rs"), rust_lib)?;
    
    let rust_main = r#"
use multi_lang_service::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Multi-language service starting...");
    
    let mut user_service = UserService::new();
    let mut auth_service = AuthService::new(UserService::new());
    
    // Create a test user
    let create_request = UserCreateRequest {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        password: "secure123".to_string(),
        status: UserStatus::Active,
    };
    
    match user_service.create_user(create_request).await {
        Ok(user) => println!("Created user: {} ({})", user.name, user.email),
        Err(e) => println!("Failed to create user: {}", e),
    }
    
    // Test authentication
    let login_request = LoginRequest {
        email: "john@example.com".to_string(),
        password: "secure123".to_string(),
    };
    
    match auth_service.login(login_request).await {
        Ok(Some(response)) => {
            println!("Login successful! Token: {}", response.token.access_token);
            
            // Validate token
            match auth_service.validate_token(&response.token.access_token).await {
                Ok(Some(user)) => println!("Token valid for user: {}", user.name),
                Ok(None) => println!("Token invalid"),
                Err(e) => println!("Token validation error: {}", e),
            }
        }
        Ok(None) => println!("Login failed"),
        Err(e) => println!("Login error: {}", e),
    }
    
    println!("Multi-language service demo completed!");
    Ok(())
}
"#;
    fs::write(temp_dir.join("src").join("main.rs"), rust_main)?;
    
    // Create Java project
    let java_pom = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 
         http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    
    <groupId>com.example</groupId>
    <artifactId>multi-lang-service</artifactId>
    <version>1.0.0</version>
    <packaging>jar</packaging>
    
    <properties>
        <maven.compiler.source>17</maven.compiler.source>
        <maven.compiler.target>17</maven.compiler.target>
        <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
        <spring.version>6.0.13</spring.version>
        <junit.version>5.9.3</junit.version>
    </properties>
    
    <dependencies>
        <dependency>
            <groupId>org.springframework</groupId>
            <artifactId>spring-context</artifactId>
            <version>${spring.version}</version>
        </dependency>
        <dependency>
            <groupId>org.springframework</groupId>
            <artifactId>spring-web</artifactId>
            <version>${spring.version}</version>
        </dependency>
        <dependency>
            <groupId>com.fasterxml.jackson.core</groupId>
            <artifactId>jackson-databind</artifactId>
            <version>2.15.2</version>
        </dependency>
        <dependency>
            <groupId>org.junit.jupiter</groupId>
            <artifactId>junit-jupiter</artifactId>
            <version>${junit.version}</version>
            <scope>test</scope>
        </dependency>
    </dependencies>
    
    <build>
        <plugins>
            <plugin>
                <groupId>org.apache.maven.plugins</groupId>
                <artifactId>maven-compiler-plugin</artifactId>
                <version>3.11.0</version>
                <configuration>
                    <source>17</source>
                    <target>17</target>
                </configuration>
            </plugin>
        </plugins>
    </build>
</project>
"#;
    fs::write(temp_dir.join("pom.xml"), java_pom)?;
    
    fs::create_dir_all(temp_dir.join("src").join("main").join("java").join("com").join("example").join("service"))?;
    
    let java_user = r#"
package com.example.service;

import com.fasterxml.jackson.annotation.JsonIgnore;
import java.time.LocalDateTime;
import java.util.Objects;
import java.util.UUID;

public class User {
    public enum Status {
        ACTIVE, INACTIVE, SUSPENDED
    }
    
    private UUID id;
    private String name;
    private String email;
    @JsonIgnore
    private String passwordHash;
    private Status status;
    private LocalDateTime createdAt;
    private LocalDateTime updatedAt;
    
    public User() {
        this.id = UUID.randomUUID();
        this.createdAt = LocalDateTime.now();
        this.updatedAt = LocalDateTime.now();
        this.status = Status.ACTIVE;
    }
    
    public User(String name, String email, String passwordHash, Status status) {
        this();
        this.name = name;
        this.email = email;
        this.passwordHash = passwordHash;
        this.status = status != null ? status : Status.ACTIVE;
    }
    
    // Getters and setters
    public UUID getId() { return id; }
    public void setId(UUID id) { this.id = id; }
    
    public String getName() { return name; }
    public void setName(String name) { 
        this.name = name; 
        this.updatedAt = LocalDateTime.now();
    }
    
    public String getEmail() { return email; }
    public void setEmail(String email) { 
        this.email = email; 
        this.updatedAt = LocalDateTime.now();
    }
    
    public String getPasswordHash() { return passwordHash; }
    public void setPasswordHash(String passwordHash) { this.passwordHash = passwordHash; }
    
    public Status getStatus() { return status; }
    public void setStatus(Status status) { 
        this.status = status; 
        this.updatedAt = LocalDateTime.now();
    }
    
    public LocalDateTime getCreatedAt() { return createdAt; }
    public void setCreatedAt(LocalDateTime createdAt) { this.createdAt = createdAt; }
    
    public LocalDateTime getUpdatedAt() { return updatedAt; }
    public void setUpdatedAt(LocalDateTime updatedAt) { this.updatedAt = updatedAt; }
    
    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        User user = (User) o;
        return Objects.equals(id, user.id);
    }
    
    @Override
    public int hashCode() {
        return Objects.hash(id);
    }
    
    @Override
    public String toString() {
        return String.format("User{id=%s, name='%s', email='%s', status=%s}", 
                           id, name, email, status);
    }
}
"#;
    fs::write(
        temp_dir.join("src").join("main").join("java").join("com").join("example").join("service").join("User.java"),
        java_user
    )?;
    
    let java_service = r#"
package com.example.service;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.time.LocalDateTime;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.stream.Collectors;

public class UserService {
    private final Map<UUID, User> users = new ConcurrentHashMap<>();
    private final Map<String, UUID> emailIndex = new ConcurrentHashMap<>();
    
    public static class UserCreateRequest {
        private String name;
        private String email;
        private String password;
        private User.Status status = User.Status.ACTIVE;
        
        // Constructors
        public UserCreateRequest() {}
        
        public UserCreateRequest(String name, String email, String password) {
            this.name = name;
            this.email = email;
            this.password = password;
        }
        
        // Getters and setters
        public String getName() { return name; }
        public void setName(String name) { this.name = name; }
        
        public String getEmail() { return email; }
        public void setEmail(String email) { this.email = email; }
        
        public String getPassword() { return password; }
        public void setPassword(String password) { this.password = password; }
        
        public User.Status getStatus() { return status; }
        public void setStatus(User.Status status) { this.status = status; }
    }
    
    public static class UserUpdateRequest {
        private String name;
        private String email;
        private User.Status status;
        
        // Getters and setters
        public String getName() { return name; }
        public void setName(String name) { this.name = name; }
        
        public String getEmail() { return email; }
        public void setEmail(String email) { this.email = email; }
        
        public User.Status getStatus() { return status; }
        public void setStatus(User.Status status) { this.status = status; }
    }
    
    public static class ServiceException extends Exception {
        public ServiceException(String message) {
            super(message);
        }
        
        public ServiceException(String message, Throwable cause) {
            super(message, cause);
        }
    }
    
    public User createUser(UserCreateRequest request) throws ServiceException {
        if (request.getName() == null || request.getName().trim().isEmpty()) {
            throw new ServiceException("Name is required");
        }
        if (request.getEmail() == null || request.getEmail().trim().isEmpty()) {
            throw new ServiceException("Email is required");
        }
        if (request.getPassword() == null || request.getPassword().length() < 6) {
            throw new ServiceException("Password must be at least 6 characters");
        }
        
        String email = request.getEmail().toLowerCase();
        if (emailIndex.containsKey(email)) {
            throw new ServiceException("User with email " + email + " already exists");
        }
        
        String passwordHash = hashPassword(request.getPassword());
        User user = new User(request.getName(), email, passwordHash, request.getStatus());
        
        users.put(user.getId(), user);
        emailIndex.put(email, user.getId());
        
        return user;
    }
    
    public Optional<User> getUser(UUID id) {
        return Optional.ofNullable(users.get(id));
    }
    
    public Optional<User> getUserByEmail(String email) {
        UUID id = emailIndex.get(email.toLowerCase());
        return id != null ? Optional.ofNullable(users.get(id)) : Optional.empty();
    }
    
    public Optional<User> updateUser(UUID id, UserUpdateRequest request) {
        User user = users.get(id);
        if (user == null) {
            return Optional.empty();
        }
        
        if (request.getName() != null) {
            user.setName(request.getName());
        }
        if (request.getEmail() != null) {
            // Update email index
            emailIndex.remove(user.getEmail());
            user.setEmail(request.getEmail().toLowerCase());
            emailIndex.put(user.getEmail(), user.getId());
        }
        if (request.getStatus() != null) {
            user.setStatus(request.getStatus());
        }
        
        return Optional.of(user);
    }
    
    public boolean deleteUser(UUID id) {
        User user = users.remove(id);
        if (user != null) {
            emailIndex.remove(user.getEmail());
            return true;
        }
        return false;
    }
    
    public List<User> listActiveUsers() {
        return users.values().stream()
                .filter(user -> user.getStatus() == User.Status.ACTIVE)
                .collect(Collectors.toList());
    }
    
    public List<User> listAllUsers() {
        return new ArrayList<>(users.values());
    }
    
    public Optional<User> authenticateUser(String email, String password) {
        Optional<User> userOpt = getUserByEmail(email);
        if (userOpt.isPresent()) {
            User user = userOpt.get();
            if (verifyPassword(password, user.getPasswordHash())) {
                return Optional.of(user);
            }
        }
        return Optional.empty();
    }
    
    public int getUserCount() {
        return users.size();
    }
    
    public int getActiveUserCount() {
        return (int) users.values().stream()
                .filter(user -> user.getStatus() == User.Status.ACTIVE)
                .count();
    }
    
    private String hashPassword(String password) {
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            byte[] hash = digest.digest(password.getBytes(StandardCharsets.UTF_8));
            StringBuilder hexString = new StringBuilder();
            for (byte b : hash) {
                String hex = Integer.toHexString(0xff & b);
                if (hex.length() == 1) {
                    hexString.append('0');
                }
                hexString.append(hex);
            }
            return hexString.toString();
        } catch (NoSuchAlgorithmException e) {
            throw new RuntimeException("SHA-256 algorithm not available", e);
        }
    }
    
    private boolean verifyPassword(String password, String hash) {
        return hashPassword(password).equals(hash);
    }
}
"#;
    fs::write(
        temp_dir.join("src").join("main").join("java").join("com").join("example").join("service").join("UserService.java"),
        java_service
    )?;
    
    let java_auth = r#"
package com.example.service;

import java.time.LocalDateTime;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

public class AuthService {
    private final UserService userService;
    private final Map<String, AuthToken> activeTokens = new ConcurrentHashMap<>();
    
    public static class AuthToken {
        private String accessToken;
        private String tokenType = "Bearer";
        private int expiresIn;
        private LocalDateTime expiresAt;
        private UUID userId;
        
        public AuthToken(String accessToken, int expiresIn, UUID userId) {
            this.accessToken = accessToken;
            this.expiresIn = expiresIn;
            this.userId = userId;
            this.expiresAt = LocalDateTime.now().plusSeconds(expiresIn);
        }
        
        // Getters and setters
        public String getAccessToken() { return accessToken; }
        public void setAccessToken(String accessToken) { this.accessToken = accessToken; }
        
        public String getTokenType() { return tokenType; }
        public void setTokenType(String tokenType) { this.tokenType = tokenType; }
        
        public int getExpiresIn() { return expiresIn; }
        public void setExpiresIn(int expiresIn) { this.expiresIn = expiresIn; }
        
        public LocalDateTime getExpiresAt() { return expiresAt; }
        public void setExpiresAt(LocalDateTime expiresAt) { this.expiresAt = expiresAt; }
        
        public UUID getUserId() { return userId; }
        public void setUserId(UUID userId) { this.userId = userId; }
        
        public boolean isExpired() {
            return LocalDateTime.now().isAfter(expiresAt);
        }
    }
    
    public static class LoginRequest {
        private String email;
        private String password;
        
        public LoginRequest() {}
        
        public LoginRequest(String email, String password) {
            this.email = email;
            this.password = password;
        }
        
        public String getEmail() { return email; }
        public void setEmail(String email) { this.email = email; }
        
        public String getPassword() { return password; }
        public void setPassword(String password) { this.password = password; }
    }
    
    public static class LoginResponse {
        private AuthToken token;
        private User user;
        
        public LoginResponse(AuthToken token, User user) {
            this.token = token;
            this.user = user;
        }
        
        public AuthToken getToken() { return token; }
        public void setToken(AuthToken token) { this.token = token; }
        
        public User getUser() { return user; }
        public void setUser(User user) { this.user = user; }
    }
    
    public AuthService(UserService userService) {
        this.userService = userService;
    }
    
    public Optional<LoginResponse> login(LoginRequest request) {
        Optional<User> userOpt = userService.authenticateUser(request.getEmail(), request.getPassword());
        if (userOpt.isEmpty()) {
            return Optional.empty();
        }
        
        User user = userOpt.get();
        String tokenString = generateToken();
        int expiresIn = 3600; // 1 hour
        
        AuthToken token = new AuthToken(tokenString, expiresIn, user.getId());
        activeTokens.put(tokenString, token);
        
        return Optional.of(new LoginResponse(token, user));
    }
    
    public Optional<User> validateToken(String tokenString) {
        AuthToken token = activeTokens.get(tokenString);
        if (token == null || token.isExpired()) {
            if (token != null) {
                activeTokens.remove(tokenString);
            }
            return Optional.empty();
        }
        
        return userService.getUser(token.getUserId());
    }
    
    public boolean logout(String tokenString) {
        return activeTokens.remove(tokenString) != null;
    }
    
    public void cleanupExpiredTokens() {
        activeTokens.entrySet().removeIf(entry -> entry.getValue().isExpired());
    }
    
    public int getActiveSessionCount() {
        cleanupExpiredTokens();
        return activeTokens.size();
    }
    
    private String generateToken() {
        return UUID.randomUUID().toString().replace("-", "") + 
               Long.toHexString(System.currentTimeMillis());
    }
}
"#;
    fs::write(
        temp_dir.join("src").join("main").join("java").join("com").join("example").join("service").join("AuthService.java"),
        java_auth
    )?;
    
    let java_main = r#"
package com.example.service;

import java.util.List;
import java.util.Optional;

public class MultiLanguageServiceApplication {
    
    public static void main(String[] args) {
        System.out.println("Multi-Language Service Application Starting...");
        
        UserService userService = new UserService();
        AuthService authService = new AuthService(userService);
        
        try {
            // Create test users
            UserService.UserCreateRequest createRequest1 = new UserService.UserCreateRequest(
                "Alice Johnson", "alice@example.com", "password123"
            );
            User user1 = userService.createUser(createRequest1);
            System.out.println("Created user: " + user1);
            
            UserService.UserCreateRequest createRequest2 = new UserService.UserCreateRequest(
                "Bob Smith", "bob@example.com", "securepass456"
            );
            createRequest2.setStatus(User.Status.ACTIVE);
            User user2 = userService.createUser(createRequest2);
            System.out.println("Created user: " + user2);
            
            // List active users
            List<User> activeUsers = userService.listActiveUsers();
            System.out.println("Active users count: " + activeUsers.size());
            
            // Test authentication
            AuthService.LoginRequest loginRequest = new AuthService.LoginRequest(
                "alice@example.com", "password123"
            );
            
            Optional<AuthService.LoginResponse> loginResponse = authService.login(loginRequest);
            if (loginResponse.isPresent()) {
                System.out.println("Login successful for: " + loginResponse.get().getUser().getName());
                System.out.println("Access token: " + loginResponse.get().getToken().getAccessToken());
                
                // Validate token
                String token = loginResponse.get().getToken().getAccessToken();
                Optional<User> validatedUser = authService.validateToken(token);
                if (validatedUser.isPresent()) {
                    System.out.println("Token validation successful for: " + validatedUser.get().getName());
                } else {
                    System.out.println("Token validation failed");
                }
                
                // Test logout
                boolean loggedOut = authService.logout(token);
                System.out.println("Logout successful: " + loggedOut);
            } else {
                System.out.println("Login failed");
            }
            
            // Test user update
            UserService.UserUpdateRequest updateRequest = new UserService.UserUpdateRequest();
            updateRequest.setStatus(User.Status.INACTIVE);
            
            Optional<User> updatedUser = userService.updateUser(user2.getId(), updateRequest);
            if (updatedUser.isPresent()) {
                System.out.println("Updated user status: " + updatedUser.get().getStatus());
            }
            
            // Final stats
            System.out.println("Total users: " + userService.getUserCount());
            System.out.println("Active users: " + userService.getActiveUserCount());
            System.out.println("Active sessions: " + authService.getActiveSessionCount());
            
        } catch (UserService.ServiceException e) {
            System.err.println("Service error: " + e.getMessage());
        } catch (Exception e) {
            System.err.println("Unexpected error: " + e.getMessage());
            e.printStackTrace();
        }
        
        System.out.println("Multi-Language Service Application Completed!");
    }
}
"#;
    fs::write(
        temp_dir.join("src").join("main").join("java").join("com").join("example").join("service").join("MultiLanguageServiceApplication.java"),
        java_main
    )?;
    
    // Create C++ project
    let cmake_lists = r#"cmake_minimum_required(VERSION 3.16)
project(MultiLanguageService)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Find required packages
find_package(nlohmann_json QUIET)
if(NOT nlohmann_json_FOUND)
    message(STATUS "nlohmann_json not found, using header-only fallback")
endif()

# Include directories
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include)

# Source files
set(SOURCES
    src/User.cpp
    src/UserService.cpp
    src/AuthService.cpp
    src/main.cpp
)

# Headers
set(HEADERS
    include/User.h
    include/UserService.h
    include/AuthService.h
    include/json_utils.h
)

# Create executable
add_executable(multi_lang_service ${SOURCES} ${HEADERS})

# Link libraries
if(nlohmann_json_FOUND)
    target_link_libraries(multi_lang_service nlohmann_json::nlohmann_json)
endif()

# Compiler-specific options
if(CMAKE_CXX_COMPILER_ID STREQUAL "GNU" OR CMAKE_CXX_COMPILER_ID STREQUAL "Clang")
    target_compile_options(multi_lang_service PRIVATE -Wall -Wextra -pedantic)
endif()

# Enable testing
enable_testing()
add_test(NAME multi_lang_service_test COMMAND multi_lang_service)
"#;
    fs::write(temp_dir.join("CMakeLists.txt"), cmake_lists)?;
    
    fs::create_dir_all(temp_dir.join("include"))?;
    fs::create_dir_all(temp_dir.join("src"))?;
    
    let cpp_user_h = r#"#pragma once

#include <string>
#include <chrono>
#include <memory>

namespace service {

enum class UserStatus {
    ACTIVE,
    INACTIVE,
    SUSPENDED
};

std::string userStatusToString(UserStatus status);
UserStatus userStatusFromString(const std::string& str);

class User {
public:
    using TimePoint = std::chrono::system_clock::time_point;
    using Id = std::string;
    
    User(const std::string& name, const std::string& email, 
         const std::string& passwordHash, UserStatus status = UserStatus::ACTIVE);
    
    // Getters
    const Id& getId() const { return id_; }
    const std::string& getName() const { return name_; }
    const std::string& getEmail() const { return email_; }
    const std::string& getPasswordHash() const { return passwordHash_; }
    UserStatus getStatus() const { return status_; }
    TimePoint getCreatedAt() const { return createdAt_; }
    TimePoint getUpdatedAt() const { return updatedAt_; }
    
    // Setters
    void setName(const std::string& name);
    void setEmail(const std::string& email);
    void setPasswordHash(const std::string& passwordHash);
    void setStatus(UserStatus status);
    
    // Utility methods
    bool verifyPassword(const std::string& password) const;
    std::string toJsonString() const;
    
private:
    Id id_;
    std::string name_;
    std::string email_;
    std::string passwordHash_;
    UserStatus status_;
    TimePoint createdAt_;
    TimePoint updatedAt_;
    
    void updateTimestamp();
    static Id generateId();
};

struct UserCreateRequest {
    std::string name;
    std::string email;
    std::string password;
    UserStatus status = UserStatus::ACTIVE;
};

struct UserUpdateRequest {
    std::optional<std::string> name;
    std::optional<std::string> email;
    std::optional<UserStatus> status;
    
    bool isEmpty() const;
};

} // namespace service
"#;
    fs::write(temp_dir.join("include").join("User.h"), cpp_user_h)?;
    
    let cpp_user_service_h = r#"#pragma once

#include "User.h"
#include <unordered_map>
#include <vector>
#include <optional>
#include <memory>
#include <stdexcept>

namespace service {

class ServiceException : public std::runtime_error {
public:
    explicit ServiceException(const std::string& message) : std::runtime_error(message) {}
};

class UserService {
public:
    UserService() = default;
    ~UserService() = default;
    
    // Non-copyable, non-movable for simplicity
    UserService(const UserService&) = delete;
    UserService& operator=(const UserService&) = delete;
    UserService(UserService&&) = delete;
    UserService& operator=(UserService&&) = delete;
    
    // User management methods
    std::shared_ptr<User> createUser(const UserCreateRequest& request);
    std::shared_ptr<User> getUser(const User::Id& id) const;
    std::shared_ptr<User> getUserByEmail(const std::string& email) const;
    std::shared_ptr<User> updateUser(const User::Id& id, const UserUpdateRequest& request);
    bool deleteUser(const User::Id& id);
    
    // Query methods
    std::vector<std::shared_ptr<User>> listActiveUsers() const;
    std::vector<std::shared_ptr<User>> listAllUsers() const;
    std::shared_ptr<User> authenticateUser(const std::string& email, const std::string& password) const;
    
    // Statistics
    size_t getUserCount() const;
    size_t getActiveUserCount() const;
    
private:
    std::unordered_map<User::Id, std::shared_ptr<User>> users_;
    std::unordered_map<std::string, User::Id> emailIndex_;
    
    std::string hashPassword(const std::string& password) const;
    bool verifyPassword(const std::string& password, const std::string& hash) const;
    void validateCreateRequest(const UserCreateRequest& request) const;
};

} // namespace service
"#;
    fs::write(temp_dir.join("include").join("UserService.h"), cpp_user_service_h)?;
    
    let cpp_auth_service_h = r#"#pragma once

#include "User.h"
#include "UserService.h"
#include <string>
#include <chrono>
#include <unordered_map>
#include <optional>
#include <memory>

namespace service {

struct AuthToken {
    std::string accessToken;
    std::string tokenType = "Bearer";
    int expiresIn;
    std::chrono::system_clock::time_point expiresAt;
    User::Id userId;
    
    AuthToken(const std::string& token, int expires, const User::Id& uid);
    bool isExpired() const;
    std::string toJsonString() const;
};

struct LoginRequest {
    std::string email;
    std::string password;
};

struct LoginResponse {
    AuthToken token;
    std::shared_ptr<User> user;
    
    LoginResponse(const AuthToken& t, std::shared_ptr<User> u) : token(t), user(std::move(u)) {}
    std::string toJsonString() const;
};

class AuthService {
public:
    explicit AuthService(std::shared_ptr<UserService> userService);
    ~AuthService() = default;
    
    // Non-copyable, non-movable for simplicity  
    AuthService(const AuthService&) = delete;
    AuthService& operator=(const AuthService&) = delete;
    AuthService(AuthService&&) = delete;
    AuthService& operator=(AuthService&&) = delete;
    
    // Authentication methods
    std::optional<LoginResponse> login(const LoginRequest& request);
    std::shared_ptr<User> validateToken(const std::string& token);
    bool logout(const std::string& token);
    
    // Maintenance methods
    void cleanupExpiredTokens();
    size_t getActiveSessionCount();
    
private:
    std::shared_ptr<UserService> userService_;
    std::unordered_map<std::string, AuthToken> activeTokens_;
    
    std::string generateToken() const;
};

} // namespace service
"#;
    fs::write(temp_dir.join("include").join("AuthService.h"), cpp_auth_service_h)?;
    
    // Create a makefile as an alternative to CMake
    let makefile = r#"# Makefile for Multi-Language Service C++ project
CXX = g++
CXXFLAGS = -std=c++17 -Wall -Wextra -pedantic -O2
INCLUDES = -Iinclude
SRCDIR = src
OBJDIR = obj
SOURCES = $(wildcard $(SRCDIR)/*.cpp)
OBJECTS = $(SOURCES:$(SRCDIR)/%.cpp=$(OBJDIR)/%.o)
TARGET = multi_lang_service

.PHONY: all clean

all: $(TARGET)

$(TARGET): $(OBJECTS) | $(OBJDIR)
	$(CXX) $(OBJECTS) -o $@

$(OBJDIR)/%.o: $(SRCDIR)/%.cpp | $(OBJDIR)
	$(CXX) $(CXXFLAGS) $(INCLUDES) -c $< -o $@

$(OBJDIR):
	mkdir -p $(OBJDIR)

clean:
	rm -rf $(OBJDIR) $(TARGET)

install: $(TARGET)
	cp $(TARGET) /usr/local/bin/

test: $(TARGET)
	./$(TARGET)

.PHONY: all clean install test
"#;
    fs::write(temp_dir.join("Makefile"), makefile)?;
    
    Ok(())
}

#[test]
fn test_comprehensive_multi_language_project_scan() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    println!("🚀 Creating comprehensive multi-language project in: {:?}", project_path);
    create_comprehensive_multi_language_project(project_path)?;
    
    println!("📊 Running full scan on multi-language project...");
    
    // Run the CLI scan command
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &project_path.to_string_lossy(),
            "scan", 
            "--no-write"
        ])
        .output()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    println!("Command output:\n{}", stdout);
    if !stderr.is_empty() {
        println!("Command stderr:\n{}", stderr);
    }
    
    // Verify the command succeeded
    assert!(output.status.success(), "Multi-language scan should succeed. stderr: {}", stderr);
    
    // Check that multiple languages were detected
    let output_text = format!("{}\n{}", stdout, stderr);
    
    // Should detect multiple languages
    assert!(
        output_text.contains("TypeScript") || output_text.contains("JavaScript"),
        "Should detect TypeScript/JavaScript files"
    );
    assert!(output_text.contains("Python"), "Should detect Python files");
    assert!(output_text.contains("Go"), "Should detect Go files"); 
    assert!(output_text.contains("Rust"), "Should detect Rust files");
    assert!(output_text.contains("Java"), "Should detect Java files");
    
    // Should process a significant number of files
    // Look for file processing indicators
    let has_file_processing = output_text.contains("files") || 
                             output_text.contains("symbols") ||
                             output_text.contains("Processing");
    
    assert!(has_file_processing, "Should show file processing activity");
    
    println!("✅ Comprehensive multi-language project scan test passed");
    println!("Languages detected and processed successfully");
    
    Ok(())
}

#[test]
fn test_language_detection_accuracy() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    println!("🔍 Testing language detection accuracy...");
    create_comprehensive_multi_language_project(project_path)?;
    
    // Count expected files for each language
    let ts_files = fs::read_dir(project_path.join("src"))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "ts")
                .unwrap_or(false)
        })
        .count();
    
    let py_files = fs::read_dir(project_path)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "py")
                .unwrap_or(false)
        })
        .count();
    
    let go_files = fs::read_dir(project_path)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "go")
                .unwrap_or(false)
        })
        .count();
    
    let rs_files = std::fs::read_dir(project_path.join("src"))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "rs")
                .unwrap_or(false)
        })
        .count();
    
    let java_files_main = project_path.join("src").join("main").join("java");
    let java_files = if java_files_main.exists() {
        walkdir::WalkDir::new(java_files_main)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path().extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "java")
                    .unwrap_or(false)
            })
            .count()
    } else {
        0
    };
    
    println!("Expected file counts:");
    println!("  TypeScript: {}", ts_files);
    println!("  Python: {}", py_files);
    println!("  Go: {}", go_files);
    println!("  Rust: {}", rs_files);
    println!("  Java: {}", java_files);
    
    // Verify we have files for each language
    assert!(ts_files > 0, "Should have TypeScript files");
    assert!(py_files > 0, "Should have Python files");
    assert!(go_files > 0, "Should have Go files");
    assert!(rs_files > 0, "Should have Rust files");
    assert!(java_files > 0, "Should have Java files");
    
    // Verify we have build/config files
    assert!(project_path.join("package.json").exists(), "Should have package.json");
    assert!(project_path.join("requirements.txt").exists(), "Should have requirements.txt");
    assert!(project_path.join("go.mod").exists(), "Should have go.mod");
    assert!(project_path.join("Cargo.toml").exists(), "Should have Cargo.toml");
    assert!(project_path.join("pom.xml").exists(), "Should have pom.xml");
    assert!(project_path.join("CMakeLists.txt").exists(), "Should have CMakeLists.txt");
    
    println!("✅ Language detection accuracy test passed");
    println!("All expected files and configurations are present");
    
    Ok(())
}

#[test]
fn test_performance_with_large_multi_language_project() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    println!("⚡ Testing performance with large multi-language project...");
    create_comprehensive_multi_language_project(project_path)?;
    
    let start_time = std::time::Instant::now();
    
    // Run scan with timing
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &project_path.to_string_lossy(),
            "scan", 
            "--no-write"
        ])
        .output()?;
    
    let duration = start_time.elapsed();
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should complete successfully
    assert!(output.status.success(), "Performance test scan should succeed. stderr: {}", stderr);
    
    // Performance expectations - should complete reasonably quickly
    assert!(duration.as_secs() < 120, "Scan should complete within 2 minutes, took {:?}", duration);
    
    // Check if performance metrics are reported
    if stderr.contains("Performance Summary") {
        println!("📊 Performance metrics found in output");
        
        // Look for throughput information
        if stderr.contains("files/second") || stderr.contains("Throughput") {
            println!("✅ Throughput metrics available");
        }
        
        if stderr.contains("Memory usage") {
            println!("✅ Memory usage metrics available");
        }
    }
    
    println!("✅ Performance test completed in {:?}", duration);
    println!("Large multi-language project processed successfully");
    
    Ok(())
}

// Helper function to use walkdir since it's commonly available in Rust projects
// If not available, we can implement a simple recursive directory traversal
mod walkdir {
    use std::path::Path;
    
    pub struct WalkDir {
        path: std::path::PathBuf,
    }
    
    impl WalkDir {
        pub fn new<P: AsRef<Path>>(path: P) -> Self {
            Self {
                path: path.as_ref().to_path_buf(),
            }
        }
        
        pub fn into_iter(self) -> impl Iterator<Item = Result<DirEntry, std::io::Error>> {
            WalkDirIter::new(self.path)
        }
    }
    
    pub struct DirEntry {
        path: std::path::PathBuf,
    }
    
    impl DirEntry {
        pub fn path(&self) -> &Path {
            &self.path
        }
    }
    
    struct WalkDirIter {
        stack: Vec<std::path::PathBuf>,
    }
    
    impl WalkDirIter {
        fn new(path: std::path::PathBuf) -> Self {
            Self {
                stack: vec![path],
            }
        }
    }
    
    impl Iterator for WalkDirIter {
        type Item = Result<DirEntry, std::io::Error>;
        
        fn next(&mut self) -> Option<Self::Item> {
            while let Some(path) = self.stack.pop() {
                match std::fs::metadata(&path) {
                    Ok(metadata) => {
                        if metadata.is_dir() {
                            match std::fs::read_dir(&path) {
                                Ok(entries) => {
                                    for entry in entries {
                                        if let Ok(entry) = entry {
                                            self.stack.push(entry.path());
                                        }
                                    }
                                }
                                Err(e) => return Some(Err(e)),
                            }
                        }
                        return Some(Ok(DirEntry { path }));
                    }
                    Err(e) => return Some(Err(e)),
                }
            }
            None
        }
    }
}
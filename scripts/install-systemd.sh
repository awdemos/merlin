#!/bin/bash

# Merlin AI Router Systemd Service Installation Script
# This script installs and configures the Merlin systemd service

set -euo pipefail

# Configuration
SERVICE_NAME="merlin"
SERVICE_USER="merlin"
SERVICE_GROUP="merlin"
BINARY_NAME="merlin"
CONFIG_DIR="/etc/merlin"
DATA_DIR="/var/lib/merlin"
LOG_DIR="/var/log/merlin"
RUN_DIR="/run/merlin"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
ENV_FILE="${CONFIG_DIR}/merlin.env"
CONF_FILE="${CONFIG_DIR}/merlin.conf"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root"
        exit 1
    fi
}

# Check system requirements
check_requirements() {
    log_info "Checking system requirements..."

    # Check if systemd is available
    if ! command -v systemctl &> /dev/null; then
        log_error "systemctl command not found. This system may not use systemd."
        exit 1
    fi

    # Check if this is a systemd-based system
    if ! pidof systemd &> /dev/null; then
        log_error "systemd is not running on this system"
        exit 1
    fi

    # Check Linux distribution
    if [[ ! -f /etc/os-release ]]; then
        log_error "Cannot determine Linux distribution"
        exit 1
    fi

    source /etc/os-release
    log_info "Detected distribution: $NAME $VERSION"

    # Check supported distributions
    case "$ID" in
        ubuntu|debian|centos|rhel|fedora)
            log_success "Supported distribution detected"
            ;;
        *)
            log_warning "Distribution $ID may not be fully supported"
            ;;
    esac
}

# Find the merlin binary
find_binary() {
    local binary_paths=(
        "./target/release/$BINARY_NAME"
        "./target/debug/$BINARY_NAME"
        "/usr/local/bin/$BINARY_NAME"
    )

    for path in "${binary_paths[@]}"; do
        if [[ -f "$path" && -x "$path" ]]; then
            echo "$path"
            return 0
        fi
    done

    log_error "Merlin binary not found. Please build it first with 'cargo build --release'"
    exit 1
}

# Create directories
create_directories() {
    log_info "Creating directories..."

    # Create configuration directory
    mkdir -p "$CONFIG_DIR"
    chmod 755 "$CONFIG_DIR"

    # Create data directory
    mkdir -p "$DATA_DIR"
    chmod 750 "$DATA_DIR"

    # Create log directory
    mkdir -p "$LOG_DIR"
    chmod 750 "$LOG_DIR"

    # Create runtime directory
    mkdir -p "$RUN_DIR"
    chmod 755 "$RUN_DIR"

    log_success "Directories created"
}

# Install service files
install_service_files() {
    log_info "Installing service files..."

    # Find the binary
    local binary_path
    binary_path=$(find_binary)
    log_info "Found binary at: $binary_path"

    # Copy binary to system location
    cp "$binary_path" "/usr/local/bin/$BINARY_NAME"
    chmod 755 "/usr/local/bin/$BINARY_NAME"

    # Install systemd service file
    if [[ -f "systemd/$SERVICE_NAME.service" ]]; then
        cp "systemd/$SERVICE_NAME.service" "$SERVICE_FILE"
        chmod 644 "$SERVICE_FILE"
        log_success "Service file installed"
    else
        log_error "Service file not found: systemd/$SERVICE_NAME.service"
        exit 1
    fi

    # Install configuration files
    if [[ -f "systemd/merlin.env" ]]; then
        cp "systemd/merlin.env" "$ENV_FILE"
        chmod 600 "$ENV_FILE"
        log_success "Environment configuration installed"
    fi

    if [[ -f "systemd/merlin.conf" ]]; then
        cp "systemd/merlin.conf" "$CONF_FILE"
        chmod 644 "$CONF_FILE"
        log_success "Daemon configuration installed"
    fi
}

# Set up permissions
setup_permissions() {
    log_info "Setting up permissions..."

    # The service uses DynamicUser=yes, so we don't need to create a static user
    # But we ensure directories have correct ownership for dynamic user management

    # Configuration directory should be owned by root
    chown root:root "$CONFIG_DIR"
    if [[ -f "$ENV_FILE" ]]; then
        chown root:root "$ENV_FILE"
    fi
    if [[ -f "$CONF_FILE" ]]; then
        chown root:root "$CONF_FILE"
    fi

    # Data and log directories will be owned by the dynamic user
    # Set proper permissions that allow the dynamic user to access them
    chown root:root "$DATA_DIR"
    chown root:root "$LOG_DIR"
    chown root:root "$RUN_DIR"

    # Ensure systemd can manage the runtime directory
    chmod 755 "$RUN_DIR"

    log_success "Permissions configured"
}

# Configure systemd
configure_systemd() {
    log_info "Configuring systemd..."

    # Reload systemd to pick up the new service
    systemctl daemon-reload

    # Enable the service to start on boot
    systemctl enable "$SERVICE_NAME"

    log_success "Systemd configured"
}

# Validate installation
validate_installation() {
    log_info "Validating installation..."

    # Check if service file is valid
    if systemctl cat "$SERVICE_NAME" > /dev/null 2>&1; then
        log_success "Service file is valid"
    else
        log_error "Service file is not valid"
        return 1
    fi

    # Check if binary is accessible
    if command -v "$BINARY_NAME" &> /dev/null; then
        log_success "Binary is accessible"
    else
        log_warning "Binary not found in PATH (this may be normal)"
    fi

    # Check Redis dependency
    if command -v redis-cli &> /dev/null; then
        if redis-cli ping > /dev/null 2>&1; then
            log_success "Redis is running and accessible"
        else
            log_warning "Redis is installed but not running. Merlin may not function correctly."
        fi
    else
        log_warning "Redis CLI not found. Please ensure Redis is installed and running for full functionality."
    fi

    # Check configuration files
    if [[ -f "$ENV_FILE" ]]; then
        log_success "Environment configuration exists"
    else
        log_warning "Environment configuration not found"
    fi

    if [[ -f "$CONF_FILE" ]]; then
        log_success "Daemon configuration exists"
    else
        log_warning "Daemon configuration not found"
    fi

    # Test service configuration
    if systemctl show "$SERVICE_NAME" > /dev/null 2>&1; then
        log_success "Service configuration is valid"
    else
        log_error "Service configuration is invalid"
        return 1
    fi
}

# Show status
show_status() {
    log_info "Installation status:"
    echo "Service file: $SERVICE_FILE - $([[ -f "$SERVICE_FILE" ]] && echo "Installed" || echo "Not found")"
    echo "Binary: /usr/local/bin/$BINARY_NAME - $([[ -f "/usr/local/bin/$BINARY_NAME" ]] && echo "Installed" || echo "Not found")"
    echo "Environment config: $ENV_FILE - $([[ -f "$ENV_FILE" ]] && echo "Installed" || echo "Not found")"
    echo "Daemon config: $CONF_FILE - $([[ -f "$CONF_FILE" ]] && echo "Installed" || echo "Not found")"
    echo "Service enabled: $(systemctl is-enabled "$SERVICE_NAME" 2>/dev/null || echo "No")"
}

# Show usage instructions
show_instructions() {
    log_success "Merlin systemd service installation completed!"
    echo
    echo "Usage instructions:"
    echo "  Start service:    systemctl start $SERVICE_NAME"
    echo "  Stop service:     systemctl stop $SERVICE_NAME"
    echo "  Restart service:  systemctl restart $SERVICE_NAME"
    echo "  Check status:     systemctl status $SERVICE_NAME"
    echo "  View logs:        journalctl -u $SERVICE_NAME -f"
    echo "  Enable service:   systemctl enable $SERVICE_NAME"
    echo "  Disable service:  systemctl disable $SERVICE_NAME"
    echo
    echo "Configuration files:"
    echo "  Environment:      $ENV_FILE"
    echo "  Daemon config:    $CONF_FILE"
    echo "  Service file:     $SERVICE_FILE"
    echo
    echo "Data directories:"
    echo "  Configuration:    $CONFIG_DIR"
    echo "  Data:            $DATA_DIR"
    echo "  Logs:            $LOG_DIR"
    echo "  Runtime:         $RUN_DIR"
    echo
    echo "Health check:"
    echo "  HTTP health:     curl http://localhost:8080/health"
    echo "  Service health:  systemctl is-active $SERVICE_NAME"
    echo
    log_warning "Before starting the service, ensure:"
    echo "  1. Configuration files are properly set up"
    echo "  2. Required ports (8080) are available"
    echo "  3. Any external dependencies (Redis, etc.) are running"
}

# Parse command line arguments
parse_args() {
    case "${1:-}" in
        --help|-h)
            echo "Usage: $0 [OPTION]"
            echo "Install Merlin systemd service"
            echo
            echo "Options:"
            echo "  --help, -h     Show this help message"
            echo "  --status       Show installation status"
            echo "  --validate     Validate existing installation"
            echo "  --uninstall    Uninstall the service"
            exit 0
            ;;
        --status)
            show_status
            exit 0
            ;;
        --validate)
            validate_installation
            exit 0
            ;;
        --uninstall)
            uninstall_service
            exit 0
            ;;
        "")
            # No arguments, proceed with installation
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
}

# Uninstall service
uninstall_service() {
    log_info "Uninstalling Merlin service..."

    # Stop and disable service
    if systemctl is-active --quiet "$SERVICE_NAME"; then
        systemctl stop "$SERVICE_NAME"
    fi

    if systemctl is-enabled --quiet "$SERVICE_NAME"; then
        systemctl disable "$SERVICE_NAME"
    fi

    # Remove service file
    if [[ -f "$SERVICE_FILE" ]]; then
        rm -f "$SERVICE_FILE"
        systemctl daemon-reload
    fi

    # Remove binary
    if [[ -f "/usr/local/bin/$BINARY_NAME" ]]; then
        rm -f "/usr/local/bin/$BINARY_NAME"
    fi

    # Remove configuration files (optional)
    read -p "Remove configuration files? (y/N): " -r
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf "$CONFIG_DIR"
        rm -rf "$DATA_DIR"
        rm -rf "$LOG_DIR"
        rm -rf "$RUN_DIR"
        log_info "Configuration files removed"
    fi

    log_success "Service uninstalled"
}

# Main installation function
main() {
    log_info "Starting Merlin systemd service installation..."

    # Parse command line arguments
    parse_args "$@"

    # Installation steps
    check_root
    check_requirements
    create_directories
    install_service_files
    setup_permissions
    configure_systemd
    validate_installation

    # Show success message and instructions
    show_instructions
}

# Run main function with all arguments
main "$@"
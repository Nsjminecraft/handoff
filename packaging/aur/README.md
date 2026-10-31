# Handoff AUR Package

## Install from AUR

```bash
yay -S handoff
# or
paru -S handoff
```

## Manual Install

```bash
git clone https://aur.archlinux.org/handoff.git
cd handoff
makepkg -si
```

## Publishing to AUR

1. Create AUR account at https://aur.archlinux.org

2. Generate SSH key (if you don't have one):
   ```bash
   ssh-keygen -t ed25519 -C "your.email@example.com"
   ```

3. Upload SSH public key to AUR account settings

4. Clone the AUR package:
   ```bash
   git clone ssh://aur@aur.archlinux.org/handoff.git
   cd handoff
   ```

5. Copy PKGBUILD and .SRCINFO:
   ```bash
   cp /path/to/this/repo/packaging/aur/PKGBUILD .
   cp /path/to/this/repo/packaging/aur/.SRCINFO .
   ```

6. Update maintainer in PKGBUILD (replace "Your Name" with your name and email)

7. Commit and push:
   ```bash
   git add PKGBUILD .SRCINFO
   git commit -m "Initial commit: handoff 0.1.0"
   git push origin master
   ```

## Updating Package

When releasing a new version:

1. Update `pkgver` in PKGBUILD
2. Update `pkgrel` to 1 (reset)
3. Regenerate .SRCINFO:
   ```bash
   makepkg --printsrcinfo > .SRCINFO
   ```
4. Commit and push to AUR

## Testing Package Locally

```bash
cd packaging/aur
makepkg -si  # Build and install
```

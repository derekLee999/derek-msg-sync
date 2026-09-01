#!/bin/zsh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="验证码接收器"
BUNDLE_IDENTIFIER="com.derek.msgsync"
VERSION="$(node -p 'JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).version' "$ROOT_DIR/src-tauri/tauri.conf.json")"

DIST_DIR="$ROOT_DIR/dist"
APP_DIR="$DIST_DIR/$APP_NAME.app"
ZIP_PATH="$DIST_DIR/$APP_NAME-$VERSION.zip"
NOTARY_ZIP_PATH="$ROOT_DIR/.build/$APP_NAME-notary.zip"
DMG_PATH="$DIST_DIR/$APP_NAME-$VERSION.dmg"
DMG_RW_PATH="$ROOT_DIR/.build/$APP_NAME-rw.dmg"
DMG_STAGING_DIR="$ROOT_DIR/.build/$APP_NAME-dmg"
DMG_MOUNT_POINT=""
TAURI_APP_PATH="$ROOT_DIR/src-tauri/target/release/bundle/macos/$APP_NAME.app"

cleanup_dmg_mount() {
    if [[ -n "${DMG_MOUNT_POINT:-}" && -d "$DMG_MOUNT_POINT" ]]; then
        /usr/bin/hdiutil detach "$DMG_MOUNT_POINT" >/dev/null 2>&1 || true
    fi
}
trap cleanup_dmg_mount EXIT

resolve_developer_dir() {
    if [[ -n "${MSGSYNC_DEVELOPER_DIR:-}" ]]; then
        [[ -d "$MSGSYNC_DEVELOPER_DIR" ]] && { printf '%s\n' "$MSGSYNC_DEVELOPER_DIR"; return 0; }
        echo "MSGSYNC_DEVELOPER_DIR 不存在: $MSGSYNC_DEVELOPER_DIR" >&2; return 1
    fi
    if [[ -d "/Applications/Xcode.app/Contents/Developer" ]]; then
        printf '%s\n' "/Applications/Xcode.app/Contents/Developer"; return 0
    fi
    local selected
    selected="$(/usr/bin/xcode-select -p 2>/dev/null || true)"
    [[ -n "$selected" && -d "$selected" ]] && { printf '%s\n' "$selected"; return 0; }
    echo "找不到可用的 Xcode developer directory,请安装 Xcode 或设置 MSGSYNC_DEVELOPER_DIR" >&2; return 1
}

resolve_sign_identity() {
    if [[ -n "${MSGSYNC_CODESIGN_IDENTITY:-}" ]]; then
        SIGN_IDENTITY="$MSGSYNC_CODESIGN_IDENTITY"
        SIGN_IDENTITY_LABEL="$MSGSYNC_CODESIGN_IDENTITY"
        return 0
    fi

    local detected
    detected="$(
        /usr/bin/security find-identity -v -p codesigning \
            | /usr/bin/sed -n 's/ *[0-9][0-9]*) \([0-9A-F]\{40\}\) "\(Developer ID Application: [^"]*\)"/\1|\2/p' \
            | /usr/bin/head -n 1
    )"

    if [[ -z "$detected" ]]; then
        echo "未找到 Developer ID Application 签名身份。请设置 MSGSYNC_CODESIGN_IDENTITY 指定身份,或用 '-' 表示 ad-hoc 签名" >&2
        return 1
    fi

    SIGN_IDENTITY="${detected%%|*}"
    SIGN_IDENTITY_LABEL="${detected#*|}"
}

resolve_notary_auth() {
    typeset -ga NOTARY_AUTH_ARGS
    NOTARY_AUTH_ARGS=()

    local keychain_profile="${NOTARY_KEYCHAIN_PROFILE:-DerekNotary}"
    if [[ -n "$keychain_profile" ]]; then
        NOTARY_AUTH_ARGS+=(--keychain-profile "$keychain_profile")
        if [[ -n "${MSGSYNC_NOTARY_KEYCHAIN:-}" ]]; then
            NOTARY_AUTH_ARGS+=(--keychain "$MSGSYNC_NOTARY_KEYCHAIN")
        fi
        return 0
    fi

    if [[ -n "${MSGSYNC_NOTARY_KEY:-}" && -n "${MSGSYNC_NOTARY_KEY_ID:-}" ]]; then
        NOTARY_AUTH_ARGS+=(--key "$MSGSYNC_NOTARY_KEY" --key-id "$MSGSYNC_NOTARY_KEY_ID")
        if [[ -n "${MSGSYNC_NOTARY_ISSUER:-}" ]]; then
            NOTARY_AUTH_ARGS+=(--issuer "$MSGSYNC_NOTARY_ISSUER")
        fi
        return 0
    fi

    if [[ -n "${MSGSYNC_NOTARY_APPLE_ID:-}" && -n "${MSGSYNC_NOTARY_TEAM_ID:-}" ]]; then
        NOTARY_AUTH_ARGS+=(--apple-id "$MSGSYNC_NOTARY_APPLE_ID" --team-id "$MSGSYNC_NOTARY_TEAM_ID")
        if [[ -n "${MSGSYNC_NOTARY_PASSWORD:-}" ]]; then
            NOTARY_AUTH_ARGS+=(--password "$MSGSYNC_NOTARY_PASSWORD")
        fi
        return 0
    fi

    echo "公证需要凭据,请设置以下任一组:NOTARY_KEYCHAIN_PROFILE、MSGSYNC_NOTARY_KEY+MSGSYNC_NOTARY_KEY_ID、MSGSYNC_NOTARY_APPLE_ID+MSGSYNC_NOTARY_TEAM_ID" >&2
    return 1
}

duration_to_seconds() {
    local duration="$1"
    local value="$duration"
    local multiplier=1

    case "$duration" in
        *s)
            value="${duration%s}"
            ;;
        *m)
            value="${duration%m}"
            multiplier=60
            ;;
        *h)
            value="${duration%h}"
            multiplier=3600
            ;;
    esac

    case "$value" in
        ''|*[!0-9]*)
            echo "无效的公证超时时间: $duration" >&2
            return 1
            ;;
    esac

    printf '%s\n' "$(( value * multiplier ))"
}

notarize_archive() {
    local archive_path="$1"
    local archive_label="$2"
    local timeout="${MSGSYNC_NOTARY_TIMEOUT:-30m}"
    local poll_interval="${MSGSYNC_NOTARY_POLL_INTERVAL_SECONDS:-15}"
    local timeout_seconds
    local submit_plist
    local info_plist
    local submission_id
    local notary_status
    local deadline

    timeout_seconds="$(duration_to_seconds "$timeout")"
    submit_plist="$(mktemp "${TMPDIR:-/tmp}/msgsync-notary-submit.XXXXXX")"
    info_plist="$(mktemp "${TMPDIR:-/tmp}/msgsync-notary-info.XXXXXX")"

    echo "提交 $archive_label 公证…" >&2
    if ! "$NOTARYTOOL_BIN" submit \
        "$archive_path" \
        "${NOTARY_AUTH_ARGS[@]}" \
        --output-format plist >"$submit_plist"; then
        rm -f "$submit_plist" "$info_plist"
        return 1
    fi

    submission_id="$(/usr/bin/plutil -extract id raw -o - "$submit_plist" 2>/dev/null || true)"
    if [[ -z "$submission_id" ]]; then
        echo "无法解析 $archive_label 的公证提交 ID" >&2
        rm -f "$submit_plist" "$info_plist"
        return 1
    fi

    echo "已收到提交 ID: $submission_id" >&2

    deadline="$(( SECONDS + timeout_seconds ))"
    while true; do
        if ! "$NOTARYTOOL_BIN" info \
            "$submission_id" \
            "${NOTARY_AUTH_ARGS[@]}" \
            --output-format plist >"$info_plist"; then
            if (( SECONDS >= deadline )); then
                echo "等待 $archive_label 公证结果超时(提交 ID: $submission_id)" >&2
                rm -f "$submit_plist" "$info_plist"
                return 1
            fi

            echo "获取 $archive_label 公证状态失败,${poll_interval}s 后重试…" >&2
            sleep "$poll_interval"
            continue
        fi

        notary_status="$(/usr/bin/plutil -extract status raw -o - "$info_plist" 2>/dev/null || true)"
        case "$notary_status" in
            Accepted)
                echo "$archive_label 公证状态: Accepted" >&2
                rm -f "$submit_plist" "$info_plist"
                return 0
                ;;
            "In Progress")
                if (( SECONDS >= deadline )); then
                    echo "等待 $archive_label 公证结果超时(提交 ID: $submission_id)" >&2
                    rm -f "$submit_plist" "$info_plist"
                    return 1
                fi

                echo "$archive_label 公证状态: In Progress(提交 ID: $submission_id)" >&2
                sleep "$poll_interval"
                ;;
            Invalid)
                echo "$archive_label 公证状态: Invalid(提交 ID: $submission_id)" >&2
                "$NOTARYTOOL_BIN" log "$submission_id" "${NOTARY_AUTH_ARGS[@]}" >&2 || true
                rm -f "$submit_plist" "$info_plist"
                return 1
                ;;
            *)
                echo "$archive_label 公证状态: ${notary_status:-unknown}(提交 ID: $submission_id)" >&2
                if (( SECONDS >= deadline )); then
                    echo "等待 $archive_label 公证结果超时(提交 ID: $submission_id)" >&2
                    rm -f "$submit_plist" "$info_plist"
                    return 1
                fi

                sleep "$poll_interval"
                ;;
        esac
    done
}

staple_path() {
    local target_path="$1"
    local target_label="$2"

    echo "为 $target_label 装订公证票据…" >&2
    "$STAPLER_BIN" staple "$target_path" >/dev/null
    "$STAPLER_BIN" validate "$target_path" >/dev/null
}

cd "$ROOT_DIR"

echo "构建 $APP_NAME $VERSION…"

if ! command -v cargo >/dev/null 2>&1; then
    [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
    echo "找不到 cargo,请先安装 Rust(https://rustup.rs)" >&2
    exit 1
fi

DEVELOPER_DIR="$(resolve_developer_dir)"
NOTARYTOOL_BIN="$DEVELOPER_DIR/usr/bin/notarytool"
STAPLER_BIN="$DEVELOPER_DIR/usr/bin/stapler"
echo "使用 Xcode developer directory: $DEVELOPER_DIR"

resolve_sign_identity
if [[ "$SIGN_IDENTITY" == "-" ]]; then
    echo "使用 ad-hoc 签名"
else
    echo "使用签名身份: $SIGN_IDENTITY_LABEL ($SIGN_IDENTITY)"
    if [[ "${MSGSYNC_SKIP_NOTARIZATION:-0}" == "1" ]]; then
        echo "MSGSYNC_SKIP_NOTARIZATION=1,跳过公证"
    else
        resolve_notary_auth
    fi
fi

if [[ ! -d "$ROOT_DIR/node_modules" ]]; then
    echo "安装前端依赖…"
    npm install
fi

echo "tauri build(release,仅 .app)…"
export APPLE_SIGNING_IDENTITY="$SIGN_IDENTITY"
npm run tauri build -- --bundles app

if [[ ! -d "$TAURI_APP_PATH" ]]; then
    echo "未找到构建产物: $TAURI_APP_PATH" >&2
    exit 1
fi

/usr/bin/codesign --verify --deep --strict "$TAURI_APP_PATH"
echo "签名验证通过"

mkdir -p "$DIST_DIR" "$ROOT_DIR/.build"
rm -rf "$APP_DIR"
/usr/bin/ditto "$TAURI_APP_PATH" "$APP_DIR"

rm -f "$ZIP_PATH" "$NOTARY_ZIP_PATH" "$DMG_PATH" "$DMG_RW_PATH"
if [[ "$SIGN_IDENTITY" != "-" && "${MSGSYNC_SKIP_NOTARIZATION:-0}" != "1" ]]; then
    /usr/bin/ditto -c -k --keepParent "$APP_DIR" "$NOTARY_ZIP_PATH"
    notarize_archive "$NOTARY_ZIP_PATH" "$APP_NAME.app"
    staple_path "$APP_DIR" "$APP_NAME.app"
    rm -f "$NOTARY_ZIP_PATH"
fi

echo "打包 $ZIP_PATH…"
/usr/bin/ditto -c -k --keepParent "$APP_DIR" "$ZIP_PATH"

echo "制作 DMG…"
rm -rf "$DMG_STAGING_DIR"
mkdir -p "$DMG_STAGING_DIR"
/usr/bin/ditto "$APP_DIR" "$DMG_STAGING_DIR/$APP_NAME.app"
ln -s /Applications "$DMG_STAGING_DIR/Applications"
/usr/bin/hdiutil create \
    -volname "$APP_NAME" \
    -srcfolder "$DMG_STAGING_DIR" \
    -ov \
    -format UDRW \
    "$DMG_RW_PATH" >/dev/null
rm -rf "$DMG_STAGING_DIR"

DMG_ATTACH_OUTPUT="$(/usr/bin/hdiutil attach -readwrite -noverify -noautoopen "$DMG_RW_PATH")"
DMG_MOUNT_POINT="$(printf '%s\n' "$DMG_ATTACH_OUTPUT" | /usr/bin/awk '/\/Volumes\// {print substr($0, index($0, "/Volumes/")); exit}')"
if [[ -z "$DMG_MOUNT_POINT" || ! -d "$DMG_MOUNT_POINT" ]]; then
    echo "挂载 DMG 失败,无法设置 Finder 布局" >&2
    exit 1
fi

/usr/bin/osascript <<APPLESCRIPT || echo "警告:DMG 布局失败,使用默认布局" >&2
set dmgFolder to POSIX file "$DMG_MOUNT_POINT" as alias
tell application "Finder"
    open dmgFolder
    set dmgWindow to container window of folder dmgFolder
    set current view of dmgWindow to icon view
    set toolbar visible of dmgWindow to false
    set statusbar visible of dmgWindow to false
    set bounds of dmgWindow to {120, 120, 780, 520}
    set arrangement of icon view options of dmgWindow to not arranged
    set icon size of icon view options of dmgWindow to 128
    set text size of icon view options of dmgWindow to 16
    set position of item "$APP_NAME.app" of folder dmgFolder to {180, 175}
    set position of item "Applications" of folder dmgFolder to {500, 175}
    update folder dmgFolder without registering applications
    delay 1
    close dmgWindow
end tell
APPLESCRIPT
/bin/sync
/usr/bin/hdiutil detach "$DMG_MOUNT_POINT" >/dev/null
DMG_MOUNT_POINT=""

/usr/bin/hdiutil convert "$DMG_RW_PATH" \
    -format UDZO \
    -imagekey zlib-level=9 \
    -o "$DMG_PATH" >/dev/null
rm -f "$DMG_RW_PATH"
if [[ "$SIGN_IDENTITY" != "-" ]]; then
    /usr/bin/codesign \
        --force \
        --sign "$SIGN_IDENTITY" \
        --timestamp \
        "$DMG_PATH" >/dev/null
    if [[ "${MSGSYNC_SKIP_NOTARIZATION:-0}" != "1" ]]; then
        notarize_archive "$DMG_PATH" "$APP_NAME.dmg"
        staple_path "$DMG_PATH" "$APP_NAME.dmg"
    fi
fi
/usr/bin/hdiutil verify "$DMG_PATH" >/dev/null
echo "DMG 校验通过"

echo "构建产物:"
echo "$APP_DIR"
echo "$ZIP_PATH"
echo "$DMG_PATH"

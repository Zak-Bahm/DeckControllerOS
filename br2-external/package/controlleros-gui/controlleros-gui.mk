################################################################################
#
# controlleros-gui
#
################################################################################

CONTROLLEROS_GUI_VERSION = 0.1.0
CONTROLLEROS_GUI_SITE = $(BR2_EXTERNAL_CONTROLLEROS_PATH)/..
CONTROLLEROS_GUI_SITE_METHOD = local
CONTROLLEROS_GUI_SUBDIR = crates/gui
CONTROLLEROS_GUI_LICENSE = UNKNOWN
CONTROLLEROS_GUI_DEPENDENCIES = mesa3d libinput libxkbcommon libdrm
CONTROLLEROS_GUI_OVERRIDE_SRCDIR_RSYNC_EXCLUSIONS = \
	--exclude .git \
	--exclude buildroot \
	--exclude out \
	--exclude target

define CONTROLLEROS_GUI_POST_RSYNC_VENDOR
	cd $(@D) && \
		mkdir -p .cargo && \
		CARGO_HOME="$(HOME)/.cargo" cargo vendor \
			--offline \
			--locked \
			--manifest-path Cargo.toml \
			VENDOR > .cargo/config.toml
endef
CONTROLLEROS_GUI_POST_RSYNC_HOOKS += CONTROLLEROS_GUI_POST_RSYNC_VENDOR

# Buildroot same-arch cross-compilation fix: glibc's linker scripts in
# the sysroot (libc.so, libm.so) contain absolute paths like
# /lib64/libc.so.6 and /usr/lib64/libc_nonshared.a. These paths do not
# exist on the host filesystem. Rust build scripts compile as host
# binaries but link against sysroot libraries via pkg-config, triggering
# these broken paths. Fix by stripping directory prefixes so the linker
# resolves them via its -L search paths.
define CONTROLLEROS_GUI_FIX_SYSROOT_LD_SCRIPTS
	for f in $(STAGING_DIR)/usr/lib/libc.so \
	         $(STAGING_DIR)/usr/lib/libm.so; do \
		if [ -f "$$f" ] && file "$$f" | grep -q text; then \
			sed -i 's|/[^ )]*[/]||g' "$$f"; \
		fi; \
	done
endef
CONTROLLEROS_GUI_PRE_BUILD_HOOKS += CONTROLLEROS_GUI_FIX_SYSROOT_LD_SCRIPTS

$(eval $(cargo-package))

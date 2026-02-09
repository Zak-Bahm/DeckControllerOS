################################################################################
#
# controllerosctl
#
################################################################################

CONTROLLEROSCTL_VERSION = 0.1.0
CONTROLLEROSCTL_SITE = $(BR2_EXTERNAL_CONTROLLEROS_PATH)/..
CONTROLLEROSCTL_SITE_METHOD = local
CONTROLLEROSCTL_SUBDIR = crates/controllerosctl
CONTROLLEROSCTL_LICENSE = UNKNOWN
CONTROLLEROSCTL_OVERRIDE_SRCDIR_RSYNC_EXCLUSIONS = \
	--exclude .git \
	--exclude buildroot \
	--exclude out \
	--exclude target

define CONTROLLEROSCTL_POST_RSYNC_VENDOR
	cd $(@D) && \
		mkdir -p .cargo && \
		CARGO_HOME="$(HOME)/.cargo" cargo vendor \
			--offline \
			--locked \
			--manifest-path Cargo.toml \
			VENDOR > .cargo/config.toml
endef
CONTROLLEROSCTL_POST_RSYNC_HOOKS += CONTROLLEROSCTL_POST_RSYNC_VENDOR

$(eval $(cargo-package))

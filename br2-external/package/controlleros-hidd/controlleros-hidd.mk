################################################################################
#
# controlleros-hidd
#
################################################################################

CONTROLLEROS_HIDD_VERSION = 0.1.0
CONTROLLEROS_HIDD_SITE = $(BR2_EXTERNAL_CONTROLLEROS_PATH)/..
CONTROLLEROS_HIDD_SITE_METHOD = local
CONTROLLEROS_HIDD_SUBDIR = crates/hidd
CONTROLLEROS_HIDD_LICENSE = UNKNOWN
CONTROLLEROS_HIDD_OVERRIDE_SRCDIR_RSYNC_EXCLUSIONS = \
	--exclude .git \
	--exclude buildroot \
	--exclude out \
	--exclude target

define CONTROLLEROS_HIDD_POST_RSYNC_VENDOR
	cd $(@D) && \
		mkdir -p .cargo && \
		CARGO_HOME="$(HOME)/.cargo" cargo vendor \
			--offline \
			--locked \
			--manifest-path Cargo.toml \
			VENDOR > .cargo/config.toml
endef
CONTROLLEROS_HIDD_POST_RSYNC_HOOKS += CONTROLLEROS_HIDD_POST_RSYNC_VENDOR

$(eval $(cargo-package))

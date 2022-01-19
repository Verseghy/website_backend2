ALTER TABLE `menu_items`
	MODIFY COLUMN `type`       VARCHAR(20)      NOT NULL,
	MODIFY COLUMN `lft`        int(10) unsigned NOT NULL,
	MODIFY COLUMN `rgt`        int(10) unsigned NOT NULL,
	MODIFY COLUMN `depth`      int(10) unsigned NOT NULL,
	MODIFY COLUMN `created_at` DATETIME         NOT NULL,
	MODIFY COLUMN `updated_at` DATETIME         NOT NULL,
	MODIFY COLUMN `deleted_at` DATETIME             NULL;

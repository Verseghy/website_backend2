ALTER TABLE `posts_data`
	MODIFY COLUMN `title`       VARCHAR(191) NOT NULL,
	MODIFY COLUMN `color`       VARCHAR(7)   NOT NULL,
	MODIFY COLUMN `images`      JSON         NOT NULL,
	MODIFY COLUMN `date`        DATETIME         NULL,
	MODIFY COLUMN `created_at`  DATETIME     NOT NULL,
	MODIFY COLUMN `updated_at`  DATETIME     NOT NULL;

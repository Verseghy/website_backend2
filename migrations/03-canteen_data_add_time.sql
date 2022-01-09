UPDATE `canteen_data` SET
	`date`       = ADDTIME(`date`, "02:00:00"),
	`created_at` = ADDTIME(`created_at`, "02:00:00"),
	`updated_at` = ADDTIME(`updated_at`, "02:00:00")
	WHERE DATE_FORMAT(`date`, '%H') = 22;

UPDATE `canteen_data` SET
	`date`       = ADDTIME(`date`, "01:00:00"),
	`created_at` = ADDTIME(`created_at`, "01:00:00"),
	`updated_at` = ADDTIME(`updated_at`, "01:00:00")
	WHERE DATE_FORMAT(`date`, '%H') = 23;

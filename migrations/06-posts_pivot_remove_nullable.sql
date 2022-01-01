ALTER TABLE `posts_pivot_labels_data`
  MODIFY COLUMN `labels_id` int(10) unsigned NOT NULL,
  MODIFY COLUMN `posts_id` int(10) unsigned NOT NULL;

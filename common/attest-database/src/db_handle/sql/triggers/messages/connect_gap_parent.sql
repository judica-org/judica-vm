/* When the new incoming message has a disconnected child,
 we update that child to have it's prev_msg_id set to match.
 N.B. the select should have a AND M.gensis_id IS NULL, but this must always be
 true so we don't need it.
 */
CREATE TRIGGER IF NOT EXISTS message_parents
AFTER
INSERT
    ON messages -- when there are messages who think this is their parent message
    WHEN EXISTS (
        SELECT
            *
        FROM
            messages M
        WHERE
            M.prev_msg = NEW.hash
    ) BEGIN
UPDATE
    messages
SET
    prev_msg_id = NEW.message_id
WHERE
    prev_msg = NEW.hash;

END;
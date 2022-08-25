SELECT M.body FROM messages M
WHERE M.genesis = :genesis
AND M.connected = 1
AND M.height > :height
ORDER BY M.height ASC
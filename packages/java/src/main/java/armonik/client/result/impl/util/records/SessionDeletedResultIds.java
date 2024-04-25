package armonik.client.result.impl.util.records;

import java.util.List;

public record SessionDeletedResultIds(String sessionId, List<String> deletedResultIds) {
}

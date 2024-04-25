package armonik.client.result.impl.util.records;

import java.util.List;

public record DeleteResultsDataResponseRecord(String sessionId, List<String> deletedResultIds) {
}

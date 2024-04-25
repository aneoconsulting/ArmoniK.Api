package armonik.client.event.impl.util.records;

import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionResponse.*;

/**
 * EventUpdateResponse represents the response containing updates for various event-related entities.
 * It encapsulates changes in task status, result status, result owner, new task, and new result.
 */
public record EventSubscriptionResponseRecord(String sessionId,
                                              TaskStatusUpdate taskStatusUpdate,
                                              ResultStatusUpdate resultStatusUpdate,
                                              ResultOwnerUpdate resultOwnerUpdate,
                                              NewTask newTask,
                                              NewResult newResult
                                  ) {
}

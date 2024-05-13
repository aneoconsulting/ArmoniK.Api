package armonik.client.event.util.records;

import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionResponse.*;

/**
 * EventSubscriptionResponseRecord represents a record containing subscription response details for an event.
 * It encapsulates various attributes related to event subscription, such as session ID, task status update,
 * result status update, result owner update, new task, and new result.
 */
public record EventSubscriptionResponseRecord(String sessionId,
                                              TaskStatusUpdate taskStatusUpdate,
                                              ResultStatusUpdate resultStatusUpdate,
                                              ResultOwnerUpdate resultOwnerUpdate,
                                              NewTask newTask,
                                              NewResult newResult
                                  ) { }

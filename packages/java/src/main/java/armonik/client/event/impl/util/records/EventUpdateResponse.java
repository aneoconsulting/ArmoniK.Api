package armonik.client.event.impl.util.records;

import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionResponse.*;

public record EventUpdateResponse(TaskStatusUpdate taskStatusUpdate,
                                  ResultStatusUpdate resultStatusUpdate,
                                  ResultOwnerUpdate resultOwnerUpdate,
                                  NewTask newTask,
                                  NewResult newResult
                                  ) {
}

package armonik.client.result.impl.util.records;

import armonik.api.grpc.v1.result_status.ResultStatusOuterClass;

import java.util.List;

public record WatchResultResponseRecord(ResultStatusOuterClass.ResultStatus resultStatus, List<String> resultIds) {
}

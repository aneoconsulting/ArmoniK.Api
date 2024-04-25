package armonik.client.result.spec;

import armonik.api.grpc.v1.results.ResultsCommon;
import armonik.api.grpc.v1.results.ResultsFilters;
import armonik.client.result.impl.util.records.DeleteResultsDataResponseRecord;
import armonik.client.result.impl.util.records.WatchResultResponseRecord;
import io.grpc.stub.StreamObserver;

import java.util.List;
import java.util.Map;

public interface IResultClientAsync {
  StreamObserver<ResultsCommon.WatchResultRequest> watchResults(StreamObserver<WatchResultResponseRecord> responseObserver);

  void getServiceConfiguration(StreamObserver<Integer> responseObserver);

  void deleteResultsData(String sessionId, List<String> resultIds, StreamObserver<DeleteResultsDataResponseRecord> responseObserver);

  void downloadResultData(String sessionId, String resultId, StreamObserver<byte[]> responseObserver);

  StreamObserver<ResultsCommon.UploadResultDataRequest> uploadResultData(String sessionId, String resultId, String payload, StreamObserver<ResultsCommon.ResultRaw> responseObserver);

  void createResults(ResultsCommon.CreateResultsRequest request, StreamObserver<Map<String, String>> responseObserver);

  void createResultsMetaData(String sessionId, List<String> names, StreamObserver<List<ResultsCommon.ResultRaw>> responseObserver);

  void getOwnerTaskId(String sessionId, List<String> resultIds, StreamObserver<Map<String, String>> responseObserver);

  void getResult(String resultId, StreamObserver<ResultsCommon.ResultRaw> responseObserver);

  void listResults(ResultsFilters.Filters filters, int total, int page, int pageSize, ResultsCommon.ListResultsRequest.Sort sort, StreamObserver<List<ResultsCommon.ResultRaw>> responseObserver);
}

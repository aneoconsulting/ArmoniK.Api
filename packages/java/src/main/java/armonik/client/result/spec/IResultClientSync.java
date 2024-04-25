package armonik.client.result.spec;

import armonik.api.grpc.v1.results.ResultsCommon;
import armonik.api.grpc.v1.results.ResultsFilters;
import armonik.client.result.impl.util.records.DeleteResultsDataResponseRecord;

import java.util.List;
import java.util.Map;

public interface IResultClientSync {
  int getServiceConfiguration();

  DeleteResultsDataResponseRecord deleteResultsData(String sessionId, List<String> resultIds);

  List<byte[]> downloadResultData(String sessionId, String resultId);

  Map<String, String> createResults(ResultsCommon.CreateResultsRequest request);

  List<ResultsCommon.ResultRaw> createResultsMetaData(String sessionId, List<String> names);

  Map<String, String> getOwnerTaskId(String sessionId, List<String> resultIds);

  ResultsCommon.ResultRaw getResult(String resultId);

  List<ResultsCommon.ResultRaw> listResults(ResultsFilters.Filters filters, int total, int page, int pageSize, ResultsCommon.ListResultsRequest.Sort sort);
}

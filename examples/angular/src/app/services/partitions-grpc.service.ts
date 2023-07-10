import { Injectable, inject } from '@angular/core';
import { ListPartitionsRequest, ListPartitionsResponse, PartitionsClient } from '@aneoconsultingfr/armonik.api.angular';
import { Observable } from 'rxjs';

@Injectable()
export class PartitionsGrpcService {
  readonly #client = inject(PartitionsClient);

  list$(): Observable<ListPartitionsResponse> {
    const options = new ListPartitionsRequest({
      page: 0,
      pageSize: 10,
      sort: {
        direction: ListPartitionsRequest.OrderDirection.ORDER_DIRECTION_ASC,
        field: ListPartitionsRequest.OrderByField.ORDER_BY_FIELD_ID
      },
      filter: {
        id: '',
        parentPartitionId: '',
        podMax: 0,
        podReserved: 0,
        preemptionPercentage: 0,
        priority: 0,
      }
    });

    return this.#client.listPartitions(options);
  }
}

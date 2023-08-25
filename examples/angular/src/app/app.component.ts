import type { AfterViewInit } from '@angular/core'
import { Component, inject } from '@angular/core'
import type { PartitionRaw } from '@aneoconsultingfr/armonik.api.angular'
import { Subject, merge, startWith, switchMap } from 'rxjs'
import { NgFor, NgIf } from '@angular/common'
import { PartitionsGrpcService } from './services/partitions-grpc.service'

@Component({
  selector: 'app-root',
  template: `
<button (click)="refresh()">Refresh</button>
<div *ngIf="loading">
  Loading...
</div>
<ul>
  <li *ngFor="let partition of partitions; trackBy:trackByPartition">
    {{ partition.id }}
  </li>
</ul>
  `,
  styles: [`
  `],
  standalone: true,
  providers: [
    PartitionsGrpcService,
  ],
  imports: [
    NgIf,
    NgFor,
  ],
})
export class AppComponent implements AfterViewInit {
  #partitionsGrpcService = inject(PartitionsGrpcService)

  #refresh$ = new Subject<void>()

  loading = true
  partitions: PartitionRaw.AsObject[] = []

  ngAfterViewInit(): void {
    merge(
      this.#refresh$,
    )
      .pipe(
        startWith({}),
        switchMap(() => {
          this.loading = true
          return this.#partitionsGrpcService.list$()
        }),
      ).subscribe(
        (response) => {
          this.loading = false

          if (response.partitions)
            this.partitions = response.partitions
        },
      )
  }

  refresh(): void {
    this.#refresh$.next()
  }

  trackByPartition(_index_: number, partition: PartitionRaw.AsObject): string {
    return partition.id
  }
}

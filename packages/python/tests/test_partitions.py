from .conftest import all_rpc_called, rpc_called, get_client
from armonik.client import ArmoniKPartitions, PartitionFieldFilter
from armonik.common import Partition


class TestArmoniKPartitions:
    def test_get_partitions(self):
        partitions_client: ArmoniKPartitions = get_client("Partitions")
        partition = partitions_client.get_partition("partition-id")

        assert rpc_called("Partitions", "GetPartition")
        assert isinstance(partition, Partition)
        assert partition.id == "partition-id"
        assert partition.parent_partition_ids == []
        assert partition.pod_reserved == 1
        assert partition.pod_max == 1
        assert partition.pod_configuration == {}
        assert partition.preemption_percentage == 0
        assert partition.priority == 1

    def test_list_partitions_no_filter(self):
        partitions_client: ArmoniKPartitions = get_client("Partitions")
        num, partitions = partitions_client.list_partitions()

        assert rpc_called("Partitions", "ListPartitions")
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert num == 0
        assert partitions == []

    def test_list_partitions_with_filter(self):
        partitions_client: ArmoniKPartitions = get_client("Partitions")
        num, partitions = partitions_client.list_partitions(PartitionFieldFilter.PRIORITY == 1)

        assert rpc_called("Partitions", "ListPartitions", 2)
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert num == 0
        assert partitions == []

    def test_service_fully_implemented(self):
        assert all_rpc_called("Partitions")
